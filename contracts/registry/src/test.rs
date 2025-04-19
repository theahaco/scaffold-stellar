use crate::{error::Error, SorobanContract__Client as SorobanContractClient};
use assert_matches::assert_matches;
use loam_sdk::soroban_sdk::{
    env, set_env,
    testutils::{Address as _, BytesN as _},
    to_string, Address, Bytes, BytesN, Env, IntoVal,
};
extern crate std;

loam_sdk::import_contract!(loam_registry);
// The contract that will be deployed by the Publisher contract.
// mod contract {
//     use loam_sdk::soroban_sdk;
//     soroban_sdk::contractimport!(file = "../../../../target/loam/loam_registry.wasm");
// }

fn init() -> (SorobanContractClient<'static>, Address) {
    set_env(Env::default());
    let env = env();
    let contract_id = Address::generate(env);
    let client =
        SorobanContractClient::new(env, &env.register_at(&contract_id, loam_registry::WASM, ()));
    let address = Address::generate(env);
    (client, address)
}

#[test]
fn handle_error_cases() {
    let (client, address) = &init();
    let env = env();

    let name = &to_string("publisher");
    assert_matches!(
        client.try_fetch(name, &None).unwrap_err(),
        Ok(Error::NoSuchVersion)
    );

    let wasm_hash = env.deployer().upload_contract_wasm(loam_registry::WASM);

    assert_matches!(
        client.try_fetch(name, &None).unwrap_err(),
        Ok(Error::NoSuchVersion)
    );

    let bytes = Bytes::from_slice(env, loam_registry::WASM);
    env.mock_all_auths();
    client.publish(name, address, &bytes, &None, &None);
    assert_eq!(client.fetch(name, &None).hash, wasm_hash);

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
    let bytes = Bytes::from_slice(env, loam_registry::WASM);
    env.mock_all_auths();
    client.publish(name, address, &bytes, &None, &None);
    let fetched_hash = client.fetch(name, &None).hash;
    let wasm_hash = env.deployer().upload_contract_wasm(loam_registry::WASM);
    assert_eq!(fetched_hash, wasm_hash);

    let second_hash: BytesN<32> = BytesN::random(env);
    client.publish_hash(name, address, &second_hash.into_val(env), &None, &None);
    let res = client.fetch(name, &None).hash;
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
