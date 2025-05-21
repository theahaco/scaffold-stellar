use crate::{
    error::Error, name::is_valid, version::Version,
    SorobanContract__Client as SorobanContractClient,
};
use assert_matches::assert_matches;
use loam_sdk::soroban_sdk::{
    self, env, set_env,
    testutils::{Address as _, BytesN as _},
    to_string, Address, Bytes, BytesN, Env, IntoVal,
};
extern crate std;

stellar_registry::import_contract_client!(stellar_registry_contract);
// Equivalent to:

// mod stellar_registry_contract {
//     use super::soroban_sdk;
//     soroban_sdk::contractimport!(file = "../../../../target/stellar/stellar_registry_contract.wasm");
// }

fn init() -> (SorobanContractClient<'static>, Address) {
    set_env(Env::default());
    let env = env();
    let contract_id = Address::generate(env);
    let address = Address::generate(env);
    let client = SorobanContractClient::new(
        env,
        &env.register_at(
            &contract_id,
            stellar_registry_contract::WASM,
            (address.clone(),),
        ),
    );
    (client, address)
}

#[test]
fn handle_error_cases() {
    let (client, address) = &init();
    let env = env();

    let name = &to_string("publisher");
    assert_matches!(
        client.try_fetch_hash(name, &None).unwrap_err(),
        Ok(Error::NoSuchContractPublished)
    );

    let wasm_hash = env
        .deployer()
        .upload_contract_wasm(stellar_registry_contract::WASM);

    assert_matches!(
        client.try_fetch_hash(name, &None).unwrap_err(),
        Ok(Error::NoSuchContractPublished)
    );

    let bytes = Bytes::from_slice(env, stellar_registry_contract::WASM);
    env.mock_all_auths();
    let version = Version::default();
    client.publish(name, address, &bytes, &version);
    assert_eq!(client.fetch_hash(name, &None), wasm_hash);

    assert_matches!(
        client
            .try_fetch_hash(name, &Some(version.publish_patch()))
            .unwrap_err(),
        Ok(Error::NoSuchVersion)
    );
    // let other_address = Address::generate(env);
    // let res = client
    //     .try_publish(name, &other_address, &bytes, &None, &None)
    //     .unwrap_err();

    // assert!(matches!(res, Ok(Error::AlreadyPublished)));
}

#[test]
fn returns_most_recent_version() {
    let (client, address) = &init();
    let env = env();
    let name = &to_string("publisher");
    // client.register_name(address, name);
    let bytes = Bytes::from_slice(env, stellar_registry_contract::WASM);
    env.mock_all_auths();
    let version = Version::default();
    client.publish(name, address, &bytes, &version);
    let fetched_hash = client.fetch_hash(name, &None);
    let wasm_hash = env
        .deployer()
        .upload_contract_wasm(stellar_registry_contract::WASM);
    assert_eq!(fetched_hash, wasm_hash);

    let second_hash: BytesN<32> = BytesN::random(env);
    client.publish_hash(
        name,
        address,
        &second_hash.into_val(env),
        &version.publish_patch(),
    );
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);

    // let third_hash: BytesN<32> = BytesN::random(env);
    // client.publish(name, &third_hash, &None, &None);
    // let res = client.fetch(name, &None);
    // assert_eq!(res, third_hash);

    // let third_hash: BytesN<32> = BytesN::random(env);
    // client.publish(name, &third_hash, &None, &None);
    // let res = client.fetch(name, &None);
    // assert_eq!(res, third_hash);
}

fn test_string(s: &str, result: bool) {
    assert!(
        is_valid(&to_string(s)) == result,
        "{s} should be {}valid",
        if !result { "in" } else { "" }
    );
}

#[test]
fn validate_names() {
    set_env(Env::default());
    test_string("publish", true);
    test_string("a_a_b", true);
    test_string("abcdefghabcdefgh", true);
    test_string("abcdefghabcdefghabcdefghabcdefgh", true);
    test_string(
        "abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh",
        true,
    );
    test_string(
        "abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgha",
        false,
    );
    test_string("a-a_b", false);
    test_string("_ab", false);
    test_string("1ab", false);
}
