extern crate std;

use crate::test::contracts::{hw_bytes, hw_bytes_v2, hw_bytes_v3, hw_hash, hw_hash_v2, hw_hash_v3};
use crate::test::util::{invalid_string, valid_string};
use crate::{
    error::Error,
    name::canonicalize,
    test::registry::{default_version, to_string, Registry},
    ContractArgs,
};
use soroban_sdk::InvokeError::Abort;
use soroban_sdk::{
    self,
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, Env, IntoVal,
};

mod contracts;
mod registry;
mod util;

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
    assert_eq!(
        client.fetch_hash(name, &Some(default_version(env))),
        registry.hash()
    );
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
    assert_eq!(
        to_string(env, "registry"),
        hw_client.hello(&to_string(env, "registry"))
    );
}

#[test]
fn hello_world_using_publish_hash() {
    let registry = &Registry::new();
    let env = registry.env();
    let client = registry.client();

    let version = registry.default_version();

    let name = &to_string(env, "contract");
    let wasm_name = &to_string(env, "wasm");

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
    assert_eq!(
        to_string(env, "registry"),
        hw_client.hello(&to_string(env, "registry"))
    );
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

    registry.mock_auth_and_deploy(author, wasm_name, name);
}

#[test]
fn returns_most_recent_version() {
    let registry = Registry::new();
    let client = &registry.client();
    let env = registry.env();
    let name = &registry.name();
    let v1 = to_string(&env, "0.0.1");
    let v2 = to_string(&env, "0.0.2");
    let v9 = to_string(&env, "0.0.9");
    let v10 = to_string(&env, "0.0.10");

    let address = registry.admin();
    registry.mock_initial_publish();
    registry.publish();
    let fetched_hash = client.fetch_hash(name, &None);
    let first_hash = registry.hash();
    assert_eq!(fetched_hash, first_hash);
    let second_hash: BytesN<32> = BytesN::random(&env);
    registry.mock_auth_for(&address, "publish_hash", (name, address, &second_hash, &v1));
    client.publish_hash(name, address, &second_hash, &v1);
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, second_hash);

    assert_eq!(
        client.try_publish_hash(name, address, &second_hash, &v2,),
        Err(Ok(Error::HashAlreadyPublished))
    );

    let third_hash: BytesN<32> = BytesN::random(&env);
    registry.mock_auth_for(&address, "publish_hash", (name, address, &third_hash, &v9));
    client.publish_hash(name, address, &third_hash, &v9);
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, third_hash);
    let forth_hash: BytesN<32> = BytesN::random(&env);
    registry.mock_auth_for(&address, "publish_hash", (name, address, &forth_hash, &v10));
    client.publish_hash(name, address, &forth_hash, &v10);

    let version = client.current_version(name);
    assert_eq!(&version, &v10);
    let res = client.fetch_hash(name, &None);
    assert_eq!(res, forth_hash);

    let res = client.fetch_hash(name, &Some(default_version(env)));
    assert_eq!(res, first_hash);
    let res = client.fetch_hash(name, &Some(v1));
    assert_eq!(res, second_hash);
    assert_eq!(
        client.try_fetch_hash(name, &Some(v2)).unwrap_err(),
        Ok(Error::NoSuchVersion)
    );
    let res = client.fetch_hash(name, &Some(v9));
    assert_eq!(res, third_hash);
    let res = client.fetch_hash(name, &Some(v10));
    assert_eq!(res, forth_hash);
}

#[test]
fn valid_simple_names() {
    valid_string("publish");
    valid_string("a_a_b");
    valid_string("abcdefghabcdefgh");
    valid_string("abcdefghabcdefghabcdefghabcdefgh");
}

#[test]
fn valid_complex_names() {
    valid_string("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh");
    valid_string("a-a_b");
}

#[test]
fn invalid_names_keywords() {
    invalid_string("pub");
    invalid_string("Pub");
    invalid_string("PUb");

    invalid_string("enum");
    invalid_string("eNum");
}

#[test]
fn invalid_names() {
    invalid_string("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgha");
    invalid_string("a-a]]]_b");
    invalid_string("_ab");
    invalid_string("-ab");
    invalid_string("1ab");
}

#[test]
fn normalization() {
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
    let too_long = &to_string(&env, "0".repeat(200).as_str());
    assert_eq!(
        client.try_publish_hash(name, address, &random_hash, too_long),
        Err(Ok(Error::InvalidVersion))
    );
    let empty = &to_string(&env, "");
    assert_eq!(
        client.try_publish_hash(name, address, &random_hash, empty),
        Err(Ok(Error::InvalidVersion))
    );
    client.publish_hash(name, address, &random_hash, new_version);
    assert_eq!(
        client.try_publish_hash(name, address, &BytesN::<32>::random(&env), version),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );
}

#[test]
fn hello_world_deploy_v2() {
    let registry = &Registry::new();
    let env = registry.env();
    let registry_client = registry.client();

    let hello_wasm = &to_string(env, "hello");
    let alice_contract = &to_string(env, "alice_serious_contract");
    let bob_contract = &to_string(env, "bobs_cool_contract");

    let v0 = &registry.default_version();
    let sv0 = &Some(registry.default_version());
    let v1 = &to_string(env, "0.0.1");
    let sv1 = &Some(v1.clone());

    let alice = &Address::generate(env);
    let bob = &Address::generate(env);

    // Step 1: Alice publishes hello_v1
    registry.mock_auth_for_publish(hello_wasm, alice, sv0, &hw_bytes(env));
    registry_client.publish(hello_wasm, alice, &hw_bytes(env), v0);
    assert_eq!(registry_client.fetch_hash(hello_wasm, &None), hw_hash(env));

    // Step 2: alice tries to publish hello_v1 with the same version and bytes, it fails
    registry.mock_auth_for_publish(hello_wasm, alice, sv0, &hw_bytes(env));
    assert_eq!(
        registry_client.try_publish(hello_wasm, alice, &hw_bytes(env), v0,),
        Err(Ok(Error::HashAlreadyPublished))
    );

    // Step 3: alice tries to publish hello_v1 with the same version and different bytes, it fails
    registry.mock_auth_for_publish(hello_wasm, alice, sv0, &hw_bytes_v2(env));
    assert_eq!(
        registry_client.try_publish(hello_wasm, alice, &hw_bytes_v2(env), v0,),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );

    // Step 4: bob tries to publish hello_v1 with a different version and different bytes, it fails
    registry.mock_auth_for_publish(hello_wasm, bob, sv1, &hw_bytes_v2(env));
    assert_eq!(
        registry_client.try_publish(hello_wasm, bob, &hw_bytes_v2(env), v1,),
        Err(Ok(Error::WasmNameAlreadyTaken))
    );

    // Step 5: alice publishes new bytes (hello_v2)
    registry.mock_auth_for_publish(hello_wasm, alice, sv1, &hw_bytes_v2(env));
    registry_client.publish(hello_wasm, alice, &hw_bytes_v2(env), v1);
    assert_eq!(
        registry_client.fetch_hash(hello_wasm, &None),
        hw_hash_v2(env)
    );

    // Step 6: bob deploys his contract using v1 bytes
    let res = registry.mock_auth_and_try_deploy(
        bob,
        &None,
        hello_wasm,
        bob_contract,
        &Some(vec![env, bob.into_val(env)]),
    );
    let address = res.unwrap().unwrap();
    let hw_client = contracts::hw_client_v2(env, &address);
    assert_eq!(hw_client.hello(), to_string(env, "hi, I'm a v2!"));

    // Step 7: alice deploys her contract using v0 bytes
    let res = registry.mock_auth_and_try_deploy(
        alice,
        sv0,
        hello_wasm,
        alice_contract,
        &Some(vec![env, alice.into_val(env)]),
    );
    let address = res.unwrap().unwrap();
    let hw_client = contracts::hw_client(env, &address);
    assert_eq!(
        to_string(env, "alice"),
        hw_client.hello(&to_string(env, "alice"))
    );
    assert_eq!(*alice, hw_client.admin());

    // Step 8: bob tries to deploy a contract using alice name
    assert_eq!(
        registry.mock_auth_and_try_deploy(
            bob,
            sv0,
            hello_wasm,
            alice_contract,
            &Some(vec![env, bob.into_val(env)])
        ),
        Err(Ok(Error::AlreadyDeployed))
    );

    // Step 9: bob tries to deploy a contract using a registry name
    assert_eq!(
        registry.mock_auth_and_try_deploy(
            bob,
            sv0,
            hello_wasm,
            &to_string(env, "registry"),
            &Some(vec![env, bob.into_val(env)])
        ),
        Err(Ok(Error::AdminOnly))
    );

    // Step 10: bob tries to upgrade alice's contract
    assert_eq!(
        registry.mock_auth_and_try_upgrade(
            bob,
            alice_contract,
            hello_wasm,
            sv1,
            &None,
            &address,
            &registry_client.fetch_hash(hello_wasm, &None)
        ),
        Err(Err(Abort)) // Abort due to bob being unauthorized to upgrade alice's contract
    );

    // Step 11: alice tries to upgrade to the latest version
    let res = registry.mock_auth_and_try_upgrade(
        alice,
        alice_contract,
        hello_wasm,
        &None,
        &None,
        &address,
        &registry_client.fetch_hash(hello_wasm, &None),
    );
    let address = res.unwrap().unwrap();
    let hw_client = contracts::hw_client_v2(env, &address);
    assert_eq!(hw_client.hello(), to_string(env, "hi, I'm a v2!"));

    // Step 12: alice rolls back to v0
    let res = registry.mock_auth_and_try_upgrade(
        alice,
        alice_contract,
        hello_wasm,
        sv0,
        &None,
        &address,
        &registry_client.fetch_hash(hello_wasm, sv0),
    );
    let address = res.unwrap().unwrap();
    let hw_client = contracts::hw_client(env, &address);
    assert_eq!(
        to_string(env, "alice"),
        hw_client.hello(&to_string(env, "alice"))
    );

    // Step 13: alice upgrades to v2 using dev_deploy
    let res = registry.mock_auth_and_try_upgrade_dev_deploy(
        alice,
        alice_contract,
        &hw_bytes_v3(env),
        &hw_hash_v3(env),
        &address,
    );
    let address = res.unwrap().unwrap();
    let hw_client = contracts::hw_client_v3(env, &address);
    assert_eq!(to_string(env, "hi, I'm a secret v3!"), hw_client.hello());

    // Step 14: alice rolls back to v0 using a custom upgrade method (and contract has no admin method)
    // TODO: auth custom upgrade method
    let res = registry.mock_auth_and_try_upgrade(
        alice,
        alice_contract,
        hello_wasm,
        sv0,
        &Some("custom_upgrade"),
        &address,
        &registry_client.fetch_hash(hello_wasm, sv0),
    );
    let address = res.unwrap().unwrap();
    let hw_client = contracts::hw_client(env, &address);
    assert_eq!(
        to_string(env, "alice"),
        hw_client.hello(&to_string(env, "alice"))
    );
    assert_eq!(*alice, hw_client.admin());
}
