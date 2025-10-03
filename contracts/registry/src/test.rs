use crate::{
    SorobanContract__, SorobanContract__Client as SorobanContractClient, error::Error,
    name::canonicalize,
};
use assert_matches::assert_matches;
use loam_sdk::soroban_sdk::{
    self, Address, Bytes, BytesN, Env, IntoVal, env, set_env,
    testutils::{Address as _, BytesN as _},
    to_string,
};
extern crate std;

fn default_version() -> soroban_sdk::String {
    to_string("0.0.0")
}

stellar_registry::import_contract_client!(registry);
// Equivalent to:

// mod registry {
//     use super::soroban_sdk;
//     soroban_sdk::contractimport!(file = "../../../../target/stellar/registry.wasm");
// }

fn init() -> (SorobanContractClient<'static>, Address) {
    set_env(Env::default());
    let env = env();
    // let contract_id = Address::generate(env);
    let address = Address::generate(env);
    let client =
        SorobanContractClient::new(env, &env.register(SorobanContract__, (address.clone(),)));
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

    let wasm_hash = env.deployer().upload_contract_wasm(registry::WASM);

    assert_matches!(
        client.try_fetch_hash(name, &None).unwrap_err(),
        Ok(Error::NoSuchContractPublished)
    );

    let bytes = Bytes::from_slice(env, registry::WASM);
    env.mock_all_auths();
    let version = default_version();
    client.publish(name, address, &bytes, &version);
    assert_eq!(client.fetch_hash(name, &None), wasm_hash);

    assert_matches!(
        client
            .try_fetch_hash(name, &Some(to_string("0.0.1")))
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
    let bytes = Bytes::from_slice(env, registry::WASM);
    env.mock_all_auths();
    let version = default_version();
    client.publish(name, address, &bytes, &version);
    let fetched_hash = client.fetch_hash(name, &None);
    let wasm_hash = env.deployer().upload_contract_wasm(registry::WASM);
    assert_eq!(fetched_hash, wasm_hash);
    let second_hash: BytesN<32> = BytesN::random(env);
    client.publish_hash(
        name,
        address,
        &second_hash.into_val(env),
        &to_string("0.0.1"),
    );
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);

    assert!(
        client
            .try_publish_hash(
                name,
                address,
                &second_hash.into_val(env),
                &to_string("0.0.2"),
            )
            .is_err()
    );

    let second_hash: BytesN<32> = BytesN::random(env);
    client.publish_hash(
        name,
        address,
        &second_hash.into_val(env),
        &to_string("0.0.9"),
    );
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);
    let second_hash: BytesN<32> = BytesN::random(env);
    client.publish_hash(
        name,
        address,
        &second_hash.into_val(env),
        &to_string("0.0.10"),
    );

    let version = client.current_version(name);
    assert_eq!(version, to_string("0.0.10"));
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);
}

fn test_string(s: &str, result: bool) {
    assert!(
        canonicalize(&to_string(s)).is_ok() == result,
        "{s} should be {}valid",
        if result { "" } else { "in" }
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
    test_string("a-a_b", true);
    test_string("a-a]]]_b", false);
    test_string("_ab", false);
    test_string("-ab", false);
    test_string("1ab", false);

    assert_eq!(
        canonicalize(&to_string("ls_test")).unwrap(),
        to_string("ls-test")
    );
    assert_eq!(
        canonicalize(&to_string("ls-test")).unwrap(),
        to_string("ls-test")
    );

    assert_eq!(canonicalize(&to_string("Test")).unwrap(), to_string("test"));
    assert_eq!(
        canonicalize(&to_string("Ls-teSt")).unwrap(),
        to_string("ls-test")
    );
}

#[test]
fn publish_to_kebab_case() {
    let (client, address) = &init();
    let env = env();
    let name = &to_string("hello_world");
    // client.register_name(address, name);
    let bytes = Bytes::from_slice(env, registry::WASM);
    env.mock_all_auths();
    let version = default_version();
    client.publish(name, address, &bytes, &version);
    let most_recent_version = client.current_version(&to_string("hello_world"));
    assert_eq!(most_recent_version, to_string("0.0.0"));
}

#[test]
fn validate_version() {
    let (client, address) = &init();
    let env = env();
    let name = &to_string("registry");
    let bytes = &Bytes::from_slice(env, registry::WASM);
    env.mock_all_auths();
    let version = &to_string("0.0.0");
    let new_version = &to_string("0.0.1");
    client.publish(name, address, bytes, version);
    let random_hash: BytesN<32> = BytesN::random(env);
    assert_eq!(
        client.try_publish_hash(name, address, &random_hash.into_val(env), version),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );
    assert_eq!(
        client.try_publish_hash(
            name,
            address,
            &random_hash.into_val(env),
            &to_string("0.  0.0"),
        ),
        Err(Ok(Error::InvalidVersion))
    );
    client.publish_hash(name, address, &random_hash.into_val(env), new_version);
    assert_eq!(
        client.try_publish_hash(
            name,
            address,
            &BytesN::<32>::random(env).into_val(env),
            version
        ),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );
}
