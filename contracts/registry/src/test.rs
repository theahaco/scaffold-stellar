extern crate std;

use crate::{
    error::Error,
    name::canonicalize,
    test::registry::{default_version, to_string, Registry},
    ContractArgs,
};
use soroban_sdk::{
    self,
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, Env, IntoVal,
};
use crate::test::contracts::{hw_bytes, hw_hash};

mod registry;
mod contracts;

#[test]
fn use_publish_method() {
    let registry = &Registry::new();
    let env = registry.env();
    let name = &registry.name();
    let client = registry.client();
    let version = registry.default_version();

    assert_eq!(
        client.try_fetch_hash(name, &None).unwrap_err(),
        Ok(Error::NoSuchWasmPublished)
    );

    registry.mock_auth_for_publish(
        name,
        registry.admin(),
        &Some(version.clone()),
        &registry.bytes(),
    );
    registry.publish();

    assert_eq!(client.fetch_hash(name, &None), registry.hash());
    assert_eq!(client.fetch_hash(name, &Some(default_version(env))), registry.hash());
    assert_eq!(client.current_version(name), default_version(env));
    assert_eq!(
        client
            .try_fetch_hash(name, &Some(to_string(env, "0.0.1")))
            .unwrap_err(),
        Ok(Error::NoSuchVersion)
    );

    let other_address = &Address::generate(env);
    let random_bytes: BytesN<32> = BytesN::random(&env);
    registry.mock_auth_for(
        other_address,
        "publish_hash",
        ContractArgs::publish_hash(name, other_address, &random_bytes, &version),
    );
    assert_eq!(
        client
            .try_publish_hash(name, other_address, &random_bytes, &version)
            .unwrap_err(),
        Ok(Error::WasmNameAlreadyTaken)
    );
}

#[test]
fn hello_world_using_publish() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash);
    let env = registry.env();
    let name = &to_string(env, "contract");
    let client = registry.client();
    let wasm_name = &registry.name();

    assert_eq!(
        client.try_fetch_contract_id(name).unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );

    let author = registry.admin();
    registry.mock_initial_publish();
    registry.publish();
    assert_eq!(client.fetch_hash(wasm_name, &None), registry.hash());

    let address = registry.mock_auth_and_deploy(author, wasm_name, name);

    assert_eq!(
        client
            .try_deploy(
                wasm_name,
                &None,
                name,
                author,
                &Some(vec![env, author.into_val(env)])
            )
            .unwrap_err(),
        Ok(Error::AlreadyDeployed)
    );

    let hw_client = contracts::hw_client(env, &address);
    assert_eq!(to_string(env, "registry"), hw_client.hello(&to_string(env, "registry")));
}

#[test]
fn hello_world_using_publish_hash() {
    let registry = &Registry::new();
    let env = registry.env();
    let client = registry.client();

    let version = registry.default_version();

    let name = &to_string(env, "contract");
    let wasm_name =&to_string(env, "wasm");

    let author = &Address::generate(env);

    env.deployer().upload_contract_wasm(hw_bytes(env));
    registry.mock_auth_for(
        author,
        "publish_hash",
        ContractArgs::publish_hash(wasm_name, author, &hw_hash(env), &version),
    );
    client.publish_hash(wasm_name, author, &hw_hash(env), &version);

    assert_eq!(client.fetch_hash(wasm_name, &None), hw_hash(env));

    let address = registry.mock_auth_and_deploy(author, wasm_name, name);

    let hw_client = contracts::hw_client(env, &address);
    assert_eq!(to_string(env, "registry"), hw_client.hello(&to_string(env, "registry")));
}

#[test]
fn contract_admin_error_cases() {
    let registry = &Registry::new();
    let env = &registry.env().clone();
    let other_address = &Address::generate(env);

    let name = &to_string(env, "registry");
    let wasm_name = &registry.name();
    let author = registry.admin();
    let client = registry.client();
    assert_eq!(
        client.try_fetch_contract_id(name).unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );
    let version = &Some(default_version(env));
    registry.mock_auth_for_publish(name, other_address, version, &registry.bytes());

    assert_eq!(
        registry.try_publish(other_address).unwrap_err(),
        Error::AdminOnly
    );
    let version = &Some(default_version(env));
    registry.mock_auth_for_publish(wasm_name, author, version, &registry.bytes());

    registry.publish();
    registry.mock_auth_for(
        other_address,
        "deploy",
        ContractArgs::deploy(
            wasm_name,
            &None,
            name,
            author,
            &Some(vec![env, other_address.into_val(env)]),
        ),
    );

    assert_eq!(
        client.try_deploy(
            wasm_name,
            &None,
            name,
            &other_address,
            &Some(vec![env, other_address.into_val(env)]),
        ),
        Err(Ok(Error::AdminOnly))
    );
}

#[test]
fn returns_most_recent_version() {
    let registry = Registry::new();
    let client = &registry.client();
    let env = registry.env();
    let name = &registry.name();
    let v1 = &to_string(&env, "0.0.1");
    let v2 = &to_string(&env, "0.0.2");
    let v9 = &to_string(&env, "0.0.9");
    let v10 = &to_string(&env, "0.0.10");

    let address = registry.admin();
    registry.mock_initial_publish();
    registry.publish();
    let fetched_hash = client.fetch_hash(name, &None);
    let wasm_hash = registry.hash();
    assert_eq!(fetched_hash, wasm_hash);
    let second_hash: BytesN<32> = BytesN::random(&env);
    registry.mock_auth_for(&address, "publish_hash", (name, address, &second_hash, v1));
    client.publish_hash(name, address, &second_hash, v1);
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);

    assert_eq!(
        client.try_publish_hash(name, address, &second_hash, v2,),
        Err(Ok(Error::HashAlreadyPublished))
    );

    let second_hash: BytesN<32> = BytesN::random(&env);
    registry.mock_auth_for(&address, "publish_hash", (name, address, &second_hash, v9));
    client.publish_hash(name, address, &second_hash, v9);
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);
    let third_hash: BytesN<32> = BytesN::random(&env);
    registry.mock_auth_for(&address, "publish_hash", (name, address, &third_hash, v10));
    client.publish_hash(name, address, &third_hash, v10);

    let version = client.current_version(name);
    assert_eq!(&version, v10);
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, third_hash);
}

#[test]
fn validate_names() {
    fn test_string(s: &str, result: bool) {
        let raw_result = canonicalize(&to_string(&Env::default(), s));
        if result {
            assert!(raw_result.is_ok(), "should be valid: {s}");
        } else {
            assert_eq!(
                raw_result,
                Err(Error::InvalidName),
                "should be invalid: {s}"
            );
        }
    }

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
        canonicalize(&to_string(&Env::default(), "ls_test")).unwrap(),
        to_string(&Env::default(), "ls-test")
    );
    assert_eq!(
        canonicalize(&to_string(&Env::default(), "ls-test")).unwrap(),
        to_string(&Env::default(), "ls-test")
    );

    assert_eq!(
        canonicalize(&to_string(&Env::default(), "Test")).unwrap(),
        to_string(&Env::default(), "test")
    );
    assert_eq!(
        canonicalize(&to_string(&Env::default(), "Ls-teSt")).unwrap(),
        to_string(&Env::default(), "ls-test")
    );
}

#[test]
fn publish_to_kebab_case() {
    let registry = &Registry::new();
    let client = registry.client();
    let address = registry.admin();
    let env = registry.env();
    let name = &to_string(&env, "hello_world");
    // client.register_name(address, name);
    let bytes = registry.bytes();
    let version = default_version(&env);
    registry.mock_auth_for_publish(name, address, &Some(version.clone()), &bytes);
    client.publish(name, address, &bytes, &version);
    let most_recent_version = client.current_version(&to_string(&env, "hello_world"));
    assert_eq!(most_recent_version, to_string(&env, "0.0.0"));
    let most_recent_version = client.current_version(&to_string(&env, "hello-world"));
    assert_eq!(most_recent_version, to_string(&env, "0.0.0"));
}

#[test]
fn validate_version() {
    let registry = &Registry::new();
    let client = registry.client();
    let address = registry.admin();
    let env = registry.env();
    let name = &to_string(&env, "registry");
    let bytes = &registry.bytes();
    env.mock_all_auths();
    let version = &to_string(&env, "0.0.0");
    let new_version = &to_string(&env, "0.0.1");
    client.publish(name, address, bytes, version);
    let random_hash: BytesN<32> = BytesN::random(&env);
    assert_eq!(
        client.try_publish_hash(name, address, &random_hash, version),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );
    assert_eq!(
        client.try_publish_hash(name, address, &random_hash, &to_string(&env, "0.  0.0"),),
        Err(Ok(Error::InvalidVersion))
    );
    client.publish_hash(name, address, &random_hash, new_version);
    assert_eq!(
        client.try_publish_hash(name, address, &BytesN::<32>::random(&env), version),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );
}
