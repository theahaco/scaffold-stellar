use crate::{
    error::Error,
    test::contracts::{hello_world, hw_bytes, hw_hash},
    test::registry::{to_string, Registry},
};
use soroban_sdk::{self, testutils::Address as _, testutils::Register, vec, Address, Vec};

fn deploy_hw(env: &soroban_sdk::Env, owner: &Address) -> Address {
    hello_world::WASM.register(env, None, hello_world::Args::__constructor(owner))
}

#[test]
fn batch_register_and_process() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();
    let owner = registry.admin();
    let addr1 = deploy_hw(env, owner);
    let addr2 = deploy_hw(env, owner);
    let addr3 = deploy_hw(env, owner);

    let entries: Vec<(soroban_sdk::String, Address, Address)> = vec![
        env,
        (to_string(env, "contract-a"), addr1.clone(), owner.clone()),
        (to_string(env, "contract-b"), addr2.clone(), owner.clone()),
        (to_string(env, "contract-c"), addr3.clone(), owner.clone()),
    ];

    client.batch_register(&entries);

    // Verify not yet registered
    assert_eq!(
        client
            .try_fetch_contract_id(&to_string(env, "contract-a"))
            .unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );

    // Process the entire batch
    let processed = client.process_batch(&3);
    assert_eq!(processed, 3);

    // Verify all are now fetchable
    assert_eq!(
        client.fetch_contract_id(&to_string(env, "contract-a")),
        addr1
    );
    assert_eq!(
        client.fetch_contract_id(&to_string(env, "contract-b")),
        addr2
    );
    assert_eq!(
        client.fetch_contract_id(&to_string(env, "contract-c")),
        addr3
    );
}

#[test]
fn batch_process_iterates() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();
    let owner = registry.admin();
    let addr1 = deploy_hw(env, owner);
    let addr2 = deploy_hw(env, owner);
    let addr3 = deploy_hw(env, owner);

    let entries: Vec<(soroban_sdk::String, Address, Address)> = vec![
        env,
        (to_string(env, "iter-a"), addr1.clone(), owner.clone()),
        (to_string(env, "iter-b"), addr2.clone(), owner.clone()),
        (to_string(env, "iter-c"), addr3.clone(), owner.clone()),
    ];

    client.batch_register(&entries);

    // Process first 2
    let processed = client.process_batch(&2);
    assert_eq!(processed, 2);

    // First two registered, third not yet
    assert_eq!(client.fetch_contract_id(&to_string(env, "iter-a")), addr1);
    assert_eq!(client.fetch_contract_id(&to_string(env, "iter-b")), addr2);
    assert_eq!(
        client
            .try_fetch_contract_id(&to_string(env, "iter-c"))
            .unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );

    // Process remaining
    let processed = client.process_batch(&2);
    assert_eq!(processed, 1);

    assert_eq!(client.fetch_contract_id(&to_string(env, "iter-c")), addr3);

    // No more pending
    assert_eq!(
        client.try_process_batch(&1).unwrap_err(),
        Ok(Error::NoPendingBatch)
    );
}

#[test]
fn batch_register_invalid_name() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();

    // "123bad" starts with digit — invalid name
    let owner = registry.admin();
    let entries: Vec<(soroban_sdk::String, Address, Address)> = vec![
        env,
        (
            to_string(env, "123bad"),
            deploy_hw(env, owner),
            owner.clone(),
        ),
    ];

    assert_eq!(
        client.try_batch_register(&entries).unwrap_err(),
        Ok(Error::InvalidName)
    );
}

#[test]
fn batch_register_already_deployed() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();

    // First register a contract normally
    let owner = registry.admin();
    let addr = deploy_hw(env, owner);
    client.register_contract(&to_string(env, "taken-name"), &addr, owner);

    // Try to batch register with same name
    let entries: Vec<(soroban_sdk::String, Address, Address)> = vec![
        env,
        (
            to_string(env, "taken-name"),
            deploy_hw(env, owner),
            owner.clone(),
        ),
    ];

    assert_eq!(
        client.try_batch_register(&entries).unwrap_err(),
        Ok(Error::AlreadyDeployed)
    );
}

#[test]
fn process_empty_batch() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();

    assert_eq!(
        client.try_process_batch(&1).unwrap_err(),
        Ok(Error::NoPendingBatch)
    );
}
