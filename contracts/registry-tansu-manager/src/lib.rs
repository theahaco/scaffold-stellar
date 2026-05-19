#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Bytes, BytesN, Env, IntoVal, String,
    Symbol, Val, Vec,
};

#[soroban_sdk_tools::scerr]
pub enum Error {
    NotApproved,
    NoOutcomeContracts,
    MultipleOutcomes,
    OutcomeTargetMismatch,
}

// Layout MUST match Consulting-Manao/tansu contracts/tansu/src/types.rs.

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
pub enum Cfg {
    Tansu,
    ProjectKey,
    Registry,
}

#[contract]
pub struct RegistryTansuManager;

#[contractimpl]
impl RegistryTansuManager {
    pub fn __constructor(env: Env, tansu: Address, project_key: Bytes, registry: Address) {
        let s = env.storage().instance();
        s.set(&Cfg::Tansu, &tansu);
        s.set(&Cfg::ProjectKey, &project_key);
        s.set(&Cfg::Registry, &registry);
    }

    pub fn tansu(env: Env) -> Address {
        env.storage().instance().get(&Cfg::Tansu).unwrap()
    }

    pub fn project_key(env: Env) -> Bytes {
        env.storage().instance().get(&Cfg::ProjectKey).unwrap()
    }

    pub fn registry(env: Env) -> Address {
        env.storage().instance().get(&Cfg::Registry).unwrap()
    }

    pub fn execute(env: Env, proposal_id: u32) -> Result<Val, Error> {
        let s = env.storage().instance();
        let tansu: Address = s.get(&Cfg::Tansu).unwrap();
        let project_key: Bytes = s.get(&Cfg::ProjectKey).unwrap();
        let registry: Address = s.get(&Cfg::Registry).unwrap();

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

        Ok(env.invoke_contract(&registry, &oc.execute_fn, oc.args))
    }
}

#[cfg(test)]
mod test;
