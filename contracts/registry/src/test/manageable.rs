use crate::{
    error::Error,
    test::contracts::{hw_bytes, hw_hash},
    test::registry::{to_string, Registry},
};
use soroban_sdk::{self, testutils::Address as _, Address};

fn setup_with_registered_contract<'a>() -> (Registry<'a>, Address) {
    let registry = Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let addr = Address::generate(env);
    env.mock_all_auths();
    let owner = registry.admin().clone();
    registry
        .client()
        .register_contract(&to_string(env, "my-contract"), &addr, &owner);
    (registry, addr)
}

#[test]
fn update_owner() {
    let (registry, _addr) = setup_with_registered_contract();
    let env = registry.env();
    let client = registry.client();

    let new_owner = Address::generate(env);
    env.mock_all_auths();
    client.update_contract_owner(&to_string(env, "my-contract"), &new_owner);

    assert_eq!(
        client.fetch_contract_owner(&to_string(env, "my-contract")),
        new_owner
    );
}

#[test]
fn update_contract_address() {
    let (registry, _addr) = setup_with_registered_contract();
    let env = registry.env();
    let client = registry.client();

    let new_address = Address::generate(env);
    env.mock_all_auths();
    client.update_contract_address(&to_string(env, "my-contract"), &new_address);

    assert_eq!(
        client.fetch_contract_id(&to_string(env, "my-contract")),
        new_address
    );
}

#[test]
fn rename_contract() {
    let (registry, addr) = setup_with_registered_contract();
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();
    client.rename_contract(&to_string(env, "my-contract"), &to_string(env, "new-name"));

    // Old name no longer works
    assert_eq!(
        client
            .try_fetch_contract_id(&to_string(env, "my-contract"))
            .unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );

    // New name works
    assert_eq!(client.fetch_contract_id(&to_string(env, "new-name")), addr);
}

#[test]
fn rename_to_taken_name() {
    let (registry, _addr) = setup_with_registered_contract();
    let env = registry.env();
    let client = registry.client();

    // Register another contract
    let addr2 = Address::generate(env);
    let owner = registry.admin();
    env.mock_all_auths();
    client.register_contract(&to_string(env, "other-contract"), &addr2, owner);

    // Try to rename to taken name
    assert_eq!(
        client
            .try_rename_contract(
                &to_string(env, "my-contract"),
                &to_string(env, "other-contract")
            )
            .unwrap_err(),
        Ok(Error::AlreadyDeployed)
    );
}

#[test]
fn update_nonexistent_contract() {
    let registry = Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let client = registry.client();

    env.mock_all_auths();

    assert_eq!(
        client
            .try_update_contract_owner(&to_string(env, "nonexistent"), &Address::generate(env))
            .unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );
}

#[test]
fn non_owner_without_manager_fails() {
    let registry = Registry::new_non_root_unmanaged();
    let env = registry.env();
    let client = registry.client();

    let addr = Address::generate(env);
    let owner = Address::generate(env);
    env.mock_all_auths();
    client.register_contract(&to_string(env, "my-contract"), &addr, &owner);

    // Non-owner trying to update (no manager, so owner auth required)
    let non_owner = Address::generate(env);
    registry.mock_auth_for(
        &non_owner,
        "update_contract_owner",
        (&to_string(env, "my-contract"), &Address::generate(env)),
    );
    assert!(client
        .try_update_contract_owner(&to_string(env, "my-contract"), &Address::generate(env))
        .is_err());
}
