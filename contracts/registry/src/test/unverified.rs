use crate::test::contracts::{
    self, hw_bytes, hw_bytes_v2, hw_bytes_v3, hw_hash, hw_hash_v2, hw_hash_v3,
};
use crate::{
    error::Error,
    test::registry::{default_version, to_string, Registry},
    ContractArgs,
};
use soroban_sdk::InvokeError;
use soroban_sdk::TryIntoVal;
use soroban_sdk::{
    self,
    testutils::{Address as _, BytesN as _},
    vec, Address, BytesN, IntoVal,
};

#[test]
fn use_publish_method() {
    let registry = &Registry::new_unverified();
    let env = registry.env();
    let name = &registry.name();
    let client = registry.client();
    let version = registry.default_version();

    let author = &Address::generate(env);

    assert_eq!(
        client.try_fetch_hash(name, &None).unwrap_err(),
        Ok(Error::NoSuchWasmPublished)
    );

    registry.mock_auth_with_addresses_for_publish(
        name,
        author,
        &Some(version.clone()),
        &registry.bytes(),
        &[author],
    );
    registry
        .try_publish(author)
        .expect("failed to publish with author");

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
    let random_bytes: BytesN<32> = BytesN::random(env);
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
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash).switch_client_to_unverified();
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
    let args = contracts::hello_world::Args::__constructor(author);

    let address = registry.mock_auth_and_deploy(author, wasm_name, name, None, &Some(args));
    registry.mock_auths_for(
        &[author, registry.admin()],
        "deploy",
        ContractArgs::deploy(
            wasm_name,
            &None,
            name,
            author,
            &Some(args.try_into_val(env).unwrap()),
            &None,
        ),
    );
    assert_eq!(
        client
            .try_deploy(
                wasm_name,
                &None,
                name,
                author,
                &Some(args.try_into_val(env).unwrap()),
                &None,
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
    let registry = &Registry::new_unverified();
    let env = registry.env();
    let client = registry.client();

    let version = registry.default_version();

    let name = &to_string(env, "contract");
    let wasm_name = &to_string(env, "wasm");

    let author = &Address::generate(env);

    env.deployer().upload_contract_wasm(hw_bytes(env));
    registry.mock_auths_for(
        &[author],
        "publish_hash",
        ContractArgs::publish_hash(wasm_name, author, &hw_hash(env), &version),
    );
    client.publish_hash(wasm_name, author, &hw_hash(env), &version);

    assert_eq!(client.fetch_hash(wasm_name, &None), hw_hash(env));

    let address = registry
        .mock_auth_with_addresses_and_try_deploy(
            author,
            &None,
            wasm_name,
            name,
            &Some(
                contracts::hello_world::Args::__constructor(author)
                    .try_into_val(env)
                    .unwrap(),
            ),
            None,
            &[author],
        )
        .unwrap()
        .unwrap();

    let hw_client = contracts::hw_client(env, &address);
    assert_eq!(
        to_string(env, "registry"),
        hw_client.hello(&to_string(env, "registry"))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn hello_world_deploy_v2() {
    let registry = &Registry::new_unverified();
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
    registry.mock_auth_with_addresses_for_publish(hello_wasm, alice, sv0, &hw_bytes(env), &[alice]);
    registry_client.publish(hello_wasm, alice, &hw_bytes(env), v0);
    assert_eq!(registry_client.fetch_hash(hello_wasm, &None), hw_hash(env));

    // Step 2: alice tries to publish hello_v1 with the same version and bytes, it fails
    registry.mock_auth_with_addresses_for_publish(hello_wasm, alice, sv0, &hw_bytes(env), &[alice]);
    assert_eq!(
        registry_client.try_publish(hello_wasm, alice, &hw_bytes(env), v0,),
        Err(Ok(Error::HashAlreadyPublished))
    );

    // Step 3: alice tries to publish hello_v1 with the same version and different bytes, it fails
    registry.mock_auth_with_addresses_for_publish(
        hello_wasm,
        alice,
        sv0,
        &hw_bytes_v2(env),
        &[alice],
    );
    assert_eq!(
        registry_client.try_publish(hello_wasm, alice, &hw_bytes_v2(env), v0,),
        Err(Ok(Error::VersionMustBeGreaterThanCurrent))
    );

    // Step 4: bob tries to publish hello_v1 with a different version and different bytes, it fails
    registry.mock_auth_with_addresses_for_publish(
        hello_wasm,
        bob,
        sv1,
        &hw_bytes_v2(env),
        &[alice],
    );
    assert_eq!(
        registry_client.try_publish(hello_wasm, bob, &hw_bytes_v2(env), v1,),
        Err(Ok(Error::WasmNameAlreadyTaken))
    );

    // Step 5: alice publishes new bytes (hello_v2)
    registry.mock_auth_with_addresses_for_publish(
        hello_wasm,
        alice,
        sv1,
        &hw_bytes_v2(env),
        &[alice],
    );
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
        &Some(vec![env, bob.to_val()]),
        None,
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
        &Some(vec![env, alice.to_val()]),
        None,
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
            &Some(vec![env, bob.into_val(env)]),
            None,
        ),
        Err(Ok(Error::AlreadyDeployed))
    );

    // Step 9: bob tries to deploy a contract using a registry name

    let contract_id = registry
        .mock_auth_with_addresses_and_try_deploy(
            bob,
            sv0,
            hello_wasm,
            &to_string(env, "registry"),
            &Some(vec![env, bob.into_val(env)]),
            None,
            &[bob],
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        contract_id,
        registry
            .client()
            .fetch_contract_id(&to_string(env, "registry"))
    );

    // Step 10: bob tries to upgrade alice's contract
    assert_eq!(
        registry.mock_auth_and_try_upgrade(
            bob,
            alice_contract,
            hello_wasm,
            sv1,
            None,
            &address,
            &registry_client.fetch_hash(hello_wasm, &None)
        ),
        Err(Err(InvokeError::Abort)) // Abort due to bob being unauthorized to upgrade alice's contract
    );

    // Step 11: alice tries to upgrade to the latest version
    let res = registry.mock_auth_and_try_upgrade(
        alice,
        alice_contract,
        hello_wasm,
        &None,
        None,
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
        None,
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
        Some("custom_upgrade"),
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

#[test]
fn hello_world_register_with_publish() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash).switch_client_to_unverified();
    let env = registry.env();
    let name = &to_string(env, "hello_world");
    let client = registry.client();

    assert_eq!(
        client.try_fetch_contract_id(name).unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );

    let author = &Address::generate(env);
    registry.mock_initial_publish();
    registry.publish();

    let wasm_hash = env.deployer().upload_contract_wasm(hw_bytes(env));
    env.mock_all_auths();
    let contract_id = env
        .deployer()
        .with_address(author.clone(), wasm_hash.clone())
        .deploy_v2(wasm_hash, (author.clone(),));
    registry.mock_auths_for(
        &[author],
        "register_contract",
        ContractArgs::register_contract(name, &contract_id, author),
    );
    registry
        .client()
        .register_contract(name, &contract_id, author);

    assert_eq!(contract_id, registry.client().fetch_contract_id(name));
}

#[test]
fn hello_world_deploy_unnamed() {
    let registry = &Registry::new_with_bytes(&hw_bytes, &hw_hash).switch_client_to_unverified();
    let env = registry.env();
    let name = &to_string(env, "hello_world");
    let client = registry.client();

    assert_eq!(
        client.try_fetch_contract_id(name).unwrap_err(),
        Ok(Error::NoSuchContractDeployed)
    );

    let author = &Address::generate(env);

    let wasm_hash = env.deployer().upload_contract_wasm(hw_bytes(env));
    let version = &Some(to_string(env, "0.0.0"));

    registry.mock_auth_with_addresses_for_publish(name, author, version, &hw_bytes(env), &[author]);
    registry
        .client()
        .publish(name, author, &hw_bytes(env), &version.clone().unwrap());
    let hash = registry.client().fetch_hash(name, &None);
    assert_eq!(hash, wasm_hash);

    let args = vec![env, author.into_val(env)];
    env.mock_all_auths();
    let contract_id =
        registry
            .client()
            .deploy_unnamed(name, &None, &Some(args), &wasm_hash.clone(), author);

    assert_eq!(
        contract_id,
        env.deployer()
            .with_address(author.clone(), wasm_hash)
            .deployed_address()
    );
}
