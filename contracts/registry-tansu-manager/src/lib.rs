#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Bytes, BytesN, Env, IntoVal, String,
    Symbol, Val, Vec,
};

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

#[contracttype]
pub enum DataKey {
    Tansu,
    ProjectKey,
    Registry,
    Executed(u32),
}

#[contract]
pub struct RegistryTansuManager;

#[contractimpl]
impl RegistryTansuManager {
    pub fn __constructor(env: Env, tansu: Address, project_key: Bytes, registry: Address) {
        let s = env.storage().instance();
        s.set(&DataKey::Tansu, &tansu);
        s.set(&DataKey::ProjectKey, &project_key);
        s.set(&DataKey::Registry, &registry);
    }

    pub fn tansu(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Tansu).unwrap()
    }

    pub fn project_key(env: Env) -> Bytes {
        env.storage().instance().get(&DataKey::ProjectKey).unwrap()
    }

    pub fn registry(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Registry).unwrap()
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
    /// Replay-protected: a successful `execute` records `DataKey::Executed(id)`
    /// permanently; later calls with the same `proposal_id` return
    /// `AlreadyExecuted`.
    ///
    /// Trust: we look up the proposal in Tansu using the stored `project_key`,
    /// so a wrong-project proposal cannot resolve. We do not re-verify
    /// `project_key` against any field of the returned proposal — Tansu's
    /// storage layout makes that lookup the only path.
    pub fn execute(env: Env, proposal_id: u32) -> Result<Val, Error> {
        let s = env.storage().persistent();
        if s.has(&DataKey::Executed(proposal_id)) {
            return Err(Error::AlreadyExecuted);
        }
        let inst = env.storage().instance();
        let tansu: Address = inst.get(&DataKey::Tansu).unwrap();
        let project_key: Bytes = inst.get(&DataKey::ProjectKey).unwrap();
        let registry: Address = inst.get(&DataKey::Registry).unwrap();

        let proposal: Proposal = env.invoke_contract(
            &tansu,
            &Symbol::new(&env, "get_proposal"),
            vec![&env, project_key.into_val(&env), proposal_id.into_val(&env)],
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
        s.set(&DataKey::Executed(proposal_id), &true);
        Ok(result)
    }
}

#[cfg(test)]
mod test;
