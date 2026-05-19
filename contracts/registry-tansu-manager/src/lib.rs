#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Bytes, BytesN, Env, IntoVal, String,
    Symbol, Val, Vec,
};
use soroban_sdk_tools::{contractstorage, InstanceItem, PersistentMap};

#[soroban_sdk_tools::scerr]
pub enum Error {
    /// Proposal exists but is not in the `Approved` state.
    NotApproved,
    /// Proposal has no outcome contracts attached.
    NoOutcomeContracts,
    /// Proposal has more than one outcome contract.
    MultipleOutcomes,
    /// Proposal's outcome targets an address other than the configured registry.
    OutcomeTargetMismatch,
    /// Proposal has already been executed by this manager.
    AlreadyExecuted,
}

// Tansu types mirrored from Consulting-Manao/tansu `contracts/tansu/src/types.rs`.
// Field order, variant order, and `#[contracttype]` annotations must stay
// identical to the upstream — Soroban encodes structs/enums by position, so
// silent drift would surface as decode failures at runtime.

#[contracttype]
#[derive(Clone)]
pub enum VoteChoice {
    Approve,
    Reject,
    Abstain,
}

#[contracttype]
#[derive(Clone)]
pub struct PublicVote {
    pub address: Address,
    pub weight: u32,
    pub vote_choice: VoteChoice,
}

#[contracttype]
#[derive(Clone)]
pub struct AnonymousVote {
    pub address: Address,
    pub weight: u32,
    pub encrypted_seeds: Vec<String>,
    pub encrypted_votes: Vec<String>,
    pub commitments: Vec<BytesN<96>>,
}

#[contracttype]
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Vote {
    PublicVote(PublicVote),
    AnonymousVote(AnonymousVote),
}

#[contracttype]
#[derive(Clone)]
pub struct VoteData {
    pub voting_ends_at: u64,
    pub public_voting: bool,
    pub token_contract: Option<Address>,
    pub votes: Vec<Vote>,
}

#[contracttype]
#[derive(Clone)]
pub enum ProposalStatus {
    Active,
    Approved,
    Rejected,
    Cancelled,
    Malicious,
}

#[contracttype]
#[derive(Clone)]
pub struct OutcomeContract {
    pub address: Address,
    pub execute_fn: Symbol,
    pub args: Vec<Val>,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u32,
    pub title: String,
    pub proposer: Address,
    pub ipfs: String,
    pub vote_data: VoteData,
    pub status: ProposalStatus,
    pub outcome_contracts: Option<Vec<OutcomeContract>>,
}

#[contractstorage(auto_shorten = true)]
pub struct Storage {
    /// Tansu DAO contract this manager queries proposals from.
    tansu: InstanceItem<Address>,
    /// Tansu workspace key this manager represents.
    project_key: InstanceItem<Bytes>,
    /// Registry contract this manager forwards approved outcomes to.
    registry: InstanceItem<Address>,
    /// Proposal IDs that have already been executed (replay guard).
    executed: PersistentMap<u32, bool>,
}

#[contract]
pub struct RegistryTansuManager;

#[contractimpl]
impl RegistryTansuManager {
    pub fn __constructor(env: &Env, tansu: &Address, project_key: &Bytes, registry: &Address) {
        Storage::set_tansu(env, tansu);
        Storage::set_project_key(env, project_key);
        Storage::set_registry(env, registry);
    }

    pub fn tansu(env: &Env) -> Address {
        Storage::get_tansu(env).unwrap()
    }

    pub fn project_key(env: &Env) -> Bytes {
        Storage::get_project_key(env).unwrap()
    }

    pub fn registry(env: &Env) -> Address {
        Storage::get_registry(env).unwrap()
    }

    /// Execute a passed Tansu proposal by forwarding its outcome to the registry.
    ///
    /// The proposal must be in `Approved` state and carry exactly one
    /// `OutcomeContract` whose `address` matches the configured registry. The
    /// outcome's `execute_fn` + `args` are forwarded via XCC — the registry's
    /// `manager.require_auth()` is satisfied automatically because this
    /// contract is the direct caller (Soroban contract-auth chains for
    /// outgoing invocations; no `authorize_as_current_contract` is needed).
    ///
    /// Replay-protected: a successful `execute` marks the proposal as
    /// executed; later calls with the same `proposal_id` return
    /// `AlreadyExecuted`.
    ///
    /// Trust: we look up the proposal in Tansu using the stored `project_key`,
    /// so a wrong-project proposal cannot resolve. We do not re-verify
    /// `project_key` against any field of the returned proposal — Tansu's
    /// storage layout makes that lookup the only path.
    pub fn execute(env: &Env, proposal_id: u32) -> Result<Val, Error> {
        if Storage::has_executed(env, &proposal_id) {
            return Err(Error::AlreadyExecuted);
        }
        let tansu = Storage::get_tansu(env).unwrap();
        let project_key = Storage::get_project_key(env).unwrap();
        let registry = Storage::get_registry(env).unwrap();

        let proposal: Proposal = env.invoke_contract(
            &tansu,
            &Symbol::new(env, "get_proposal"),
            vec![env, project_key.into_val(env), proposal_id.into_val(env)],
        );

        if !matches!(proposal.status, ProposalStatus::Approved) {
            return Err(Error::NotApproved);
        }
        let outcomes = proposal
            .outcome_contracts
            .ok_or(Error::NoOutcomeContracts)?;
        if outcomes.len() != 1 {
            return Err(Error::MultipleOutcomes);
        }
        let oc = outcomes.get(0).unwrap();
        if oc.address != registry {
            return Err(Error::OutcomeTargetMismatch);
        }

        let result: Val = env.invoke_contract(&registry, &oc.execute_fn, oc.args);
        Storage::set_executed(env, &proposal_id, &true);
        Ok(result)
    }
}

#[cfg(test)]
mod test;
