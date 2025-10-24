
extern crate std;

use soroban_sdk::{self, Address, Bytes, BytesN, Env};

mod hello_world {
    use super::soroban_sdk;
    soroban_sdk::contractimport!(file = "../../target/stellar/local/hello_world.wasm");
}

mod hello_world_v2 {
    use super::soroban_sdk;
    soroban_sdk::contractimport!(file = "../../target/stellar/local/hello_world_v2.wasm");
}

mod hello_world_v3 {
    use super::soroban_sdk;
    soroban_sdk::contractimport!(file = "../../target/stellar/local/hello_world_v3.wasm");
}

const HW_WASM: &[u8] = include_bytes!("../../../../target/stellar/local/hello_world.wasm");
const HW_WASM_V2: &[u8] = include_bytes!("../../../../target/stellar/local/hello_world_v2.wasm");
const HW_WASM_V3: &[u8] = include_bytes!("../../../../target/stellar/local/hello_world_v3.wasm");

pub fn hw_hash(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(HW_WASM)
}
pub fn hw_client<'a>(env: &Env, address: &Address) -> hello_world::Client<'a> {
    hello_world::Client::new(env, address)
}
pub fn hw_bytes(env: &Env) -> Bytes {
    Bytes::from_slice(env, HW_WASM)
}

pub fn hw_hash_v2(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(HW_WASM_V2)
}
pub fn hw_client_v2<'a>(env: &Env, address: &Address) -> hello_world_v2::Client<'a> {
    hello_world_v2::Client::new(env, address)
}
pub fn hw_bytes_v2(env: &Env) -> Bytes {
    Bytes::from_slice(env, HW_WASM_V2)
}

pub fn hw_client_v3<'a>(env: &Env, address: &Address) -> hello_world_v3::Client<'a> {
    hello_world_v3::Client::new(env, address)
}
pub fn hw_bytes_v3(env: &Env) -> Bytes {
    Bytes::from_slice(env, HW_WASM_V3)
}