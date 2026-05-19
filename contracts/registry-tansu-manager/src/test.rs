extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, testutils::Address as _, vec, Address,
    Bytes, Env, IntoVal, String, Symbol, Vec,
};
use soroban_sdk_tools::auth::setup_mock_auth;

use crate::{
    Error, OutcomeContract, Proposal, ProposalStatus, RegistryTansuManager,
    RegistryTansuManagerClient, VoteData,
};

// Stub Tansu contract: stores one Proposal under (project_key, id) and returns it on get_proposal.

#[contracttype]
enum TansuStubKey {
    Proposal(Bytes, u32),
}

#[contract]
pub struct TansuStub;

#[contractimpl]
impl TansuStub {
    pub fn set_proposal(env: Env, project_key: Bytes, proposal: Proposal) {
        env.storage()
            .instance()
            .set(&TansuStubKey::Proposal(project_key, proposal.id), &proposal);
    }

    pub fn get_proposal(env: Env, project_key: Bytes, proposal_id: u32) -> Proposal {
        env.storage()
            .instance()
            .get(&TansuStubKey::Proposal(project_key, proposal_id))
            .unwrap()
    }
}

// Stub registry: requires manager auth on `manager_only`, records the value.

#[contracttype]
enum RegStubKey {
    Manager,
    Recorded,
}

#[contract]
pub struct RegistryStub;

#[contractimpl]
impl RegistryStub {
    pub fn __constructor(env: Env, manager: Address) {
        env.storage().instance().set(&RegStubKey::Manager, &manager);
    }

    pub fn manager_only(env: Env, value: u32) -> u32 {
        let manager: Address = env.storage().instance().get(&RegStubKey::Manager).unwrap();
        manager.require_auth();
        env.storage().instance().set(&RegStubKey::Recorded, &value);
        value
    }

    pub fn recorded(env: Env) -> Option<u32> {
        env.storage().instance().get(&RegStubKey::Recorded)
    }
}

// ---------------------------------------------------------------------------
// Test scaffolding
// ---------------------------------------------------------------------------

struct Setup {
    env: Env,
    project_key: Bytes,
    tansu: Address,
    registry: Address,
    #[allow(dead_code)]
    manager: Address,
    manager_client: RegistryTansuManagerClient<'static>,
}

fn setup() -> Setup {
    let env = Env::default();
    let project_key = Bytes::from_slice(&env, &[7u8; 16]);
    let tansu = env.register(TansuStub, ());

    // Pre-compute the manager address so the registry can be constructed with it
    // as `manager`. (Registers a transient placeholder, then registers the real
    // manager contract at a deterministic address derived from arg hash — simpler:
    // register the manager first, then the registry with the manager address.)
    let manager = env.register(
        RegistryTansuManager,
        (
            tansu.clone(),
            project_key.clone(),
            // dummy registry address; rewritten via instance storage below
            Address::generate(&env),
        ),
    );
    let registry = env.register(RegistryStub, (manager.clone(),));

    // Patch the manager's stored registry to the real RegistryStub address.
    env.as_contract(&manager, || {
        crate::Storage::set_registry(&env, &registry);
    });

    let manager_client = RegistryTansuManagerClient::new(&env, &manager);

    Setup {
        env,
        project_key,
        tansu,
        registry,
        manager,
        manager_client,
    }
}

fn empty_vote_data(env: &Env) -> VoteData {
    VoteData {
        voting_ends_at: 0,
        public_voting: true,
        token_contract: None,
        votes: Vec::new(env),
    }
}

fn plant_proposal(
    env: &Env,
    tansu: &Address,
    project_key: &Bytes,
    id: u32,
    status: ProposalStatus,
    outcomes: Option<Vec<OutcomeContract>>,
) {
    let proposal = Proposal {
        id,
        title: String::from_str(env, "t"),
        proposer: Address::generate(env),
        ipfs: String::from_str(env, ""),
        vote_data: empty_vote_data(env),
        status,
        outcome_contracts: outcomes,
    };
    let client = TansuStubClient::new(env, tansu);
    client.set_proposal(project_key, &proposal);
}

fn one_outcome(env: &Env, registry: &Address, value: u32) -> Vec<OutcomeContract> {
    vec![
        env,
        OutcomeContract {
            address: registry.clone(),
            execute_fn: symbol_short!("man_only"),
            args: vec![env, value.into_val(env)],
        },
    ]
}

// ---------------------------------------------------------------------------
// Happy path: approved proposal -> registry call succeeds via contract auth
// ---------------------------------------------------------------------------

#[test]
fn approved_proposal_forwards_to_registry() {
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 42u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    // No external signer needed: the manager contract's auth satisfies
    // registry's `manager.require_auth()` via the XCC contract-auth chain.
    let result: u32 = s
        .manager_client
        .execute(&1)
        .try_into()
        .expect("Val should decode to u32");

    assert_eq!(result, 42);

    let reg = RegistryStubClient::new(&s.env, &s.registry);
    assert_eq!(reg.recorded(), Some(42));
}

// ---------------------------------------------------------------------------
// Negative cases
// ---------------------------------------------------------------------------

#[test]
fn active_proposal_is_rejected() {
    let s = setup();
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Active,
        Some(one_outcome(&s.env, &s.registry, 1)),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::NotApproved);
}

#[test]
fn rejected_proposal_is_rejected() {
    let s = setup();
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Rejected,
        Some(one_outcome(&s.env, &s.registry, 1)),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::NotApproved);
}

#[test]
fn proposal_without_outcomes_is_rejected() {
    let s = setup();
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        None,
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::NoOutcomeContracts);
}

#[test]
fn proposal_with_multiple_outcomes_is_rejected() {
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 1u32.into_val(&s.env)],
        },
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 2u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::MultipleOutcomes);
}

#[test]
fn proposal_targeting_wrong_address_is_rejected() {
    let s = setup();
    let wrong = Address::generate(&s.env);
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: wrong,
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 1u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::OutcomeTargetMismatch);
}

// ---------------------------------------------------------------------------
// Auth-flow guard: the registry's manager-only function must reject calls
// that come from somewhere other than the manager contract.
// ---------------------------------------------------------------------------

#[test]
fn approved_proposal_cannot_be_replayed() {
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.registry.clone(),
            execute_fn: Symbol::new(&s.env, "manager_only"),
            args: vec![&s.env, 7u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    s.manager_client.execute(&1);
    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::AlreadyExecuted);
}

#[test]
fn proposal_targeting_manager_itself_is_rejected() {
    // An attacker-crafted proposal whose outcome address is the manager
    // contract (not the registry) must be rejected — otherwise the manager
    // could be tricked into recursively re-entering itself.
    let s = setup();
    let outcomes = vec![
        &s.env,
        OutcomeContract {
            address: s.manager.clone(),
            execute_fn: Symbol::new(&s.env, "execute"),
            args: vec![&s.env, 1u32.into_val(&s.env)],
        },
    ];
    plant_proposal(
        &s.env,
        &s.tansu,
        &s.project_key,
        1,
        ProposalStatus::Approved,
        Some(outcomes),
    );

    let err = s.manager_client.try_execute(&1).err().unwrap().unwrap();
    assert_eq!(err, Error::OutcomeTargetMismatch);
}

#[test]
#[should_panic] // require_auth on the manager address fails for an outside caller
fn registry_rejects_direct_caller() {
    let s = setup();
    let outsider = Address::generate(&s.env);

    // Authorize the outsider (not the manager contract) and call directly.
    setup_mock_auth(&s.env, &s.registry, "manager_only", (99u32,), &[&outsider]);
    let reg = RegistryStubClient::new(&s.env, &s.registry);
    reg.manager_only(&99u32);
}
