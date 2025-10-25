extern crate std;

use soroban_sdk::{self, Address, Bytes, BytesN, Env};

mod hello_world {
    soroban_sdk::contractimport!(file = "../../target/stellar/local/hello_world.wasm");
}

mod hello_world_v2 {
    soroban_sdk::contractimport!(file = "../../target/stellar/local/hello_world_v2.wasm");
}

mod hello_world_v3 {
    soroban_sdk::contractimport!(file = "../../target/stellar/local/hello_world_v3.wasm");
}


pub fn hw_hash(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(hello_world::WASM)
}
pub fn hw_client<'a>(env: &Env, address: &Address) -> hello_world::Client<'a> {
    hello_world::Client::new(env, address)
}
pub fn hw_bytes(env: &Env) -> Bytes {
    Bytes::from_slice(env, hello_world::WASM)
}

pub fn hw_hash_v2(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(hello_world_v2::WASM)
}
pub fn hw_client_v2<'a>(env: &Env, address: &Address) -> hello_world_v2::Client<'a> {
    hello_world_v2::Client::new(env, address)
}
pub fn hw_bytes_v2(env: &Env) -> Bytes {
    Bytes::from_slice(env, hello_world_v2::WASM)
}

pub fn hw_hash_v3(env: &Env) -> BytesN<32> {
    env.deployer().upload_contract_wasm(hello_world_v3::WASM)
}
pub fn hw_client_v3<'a>(env: &Env, address: &Address) -> hello_world_v3::Client<'a> {
    hello_world_v3::Client::new(env, address)
}
pub fn hw_bytes_v3(env: &Env) -> Bytes {
    Bytes::from_slice(env, hello_world_v3::WASM)
}
