use crate::{
    error::Error,
    test::contracts::{hw_bytes, hw_hash},
    test::registry::{to_string, Registry},
};
use soroban_sdk::{self, testutils::Address as _, vec, Address, Vec};

#[test]
fn batch_register_and_process() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    // Deploy 3 contracts externally
    env.mock_all_auths();
    let addr1 = Address::generate(env);
    let addr2 = Address::generate(env);
    let addr3 = Address::generate(env);
    let owner = registry.admin();

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

    // Process the batch
    let processed = client.process_batch();
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
fn batch_register_invalid_name() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();

    // "123bad" starts with digit — invalid name
    let entries: Vec<(soroban_sdk::String, Address, Address)> = vec![
        env,
        (
            to_string(env, "123bad"),
            Address::generate(env),
            registry.admin().clone(),
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
    let addr = Address::generate(env);
    let owner = registry.admin();
    client.register_contract(&to_string(env, "taken-name"), &addr, owner);

    // Try to batch register with same name
    let entries: Vec<(soroban_sdk::String, Address, Address)> = vec![
        env,
        (
            to_string(env, "taken-name"),
            Address::generate(env),
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
        client.try_process_batch().unwrap_err(),
        Ok(Error::NoPendingBatch)
    );
}
