
extern crate std;

use soroban_sdk::{self, Address, Bytes, BytesN, Env};
use crate::test::contracts::hello_world::Client;

mod hello_world {
    use super::soroban_sdk;
    soroban_sdk::contractimport!(file = "../../target/stellar/hello_world.wasm");
}

const HW_WASM: &[u8] = include_bytes!("../../../../target/stellar/hello_world.wasm");

pub fn hw_hash(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(HW_WASM)
}

pub fn hw_client<'a>(env: &Env, address: &Address) -> Client<'a> {
    Client::new(env, address)
}

pub fn hw_bytes(env: &Env) -> Bytes {
    Bytes::from_slice(env, HW_WASM)
}