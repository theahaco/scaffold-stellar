#![no_std]
use soroban_sdk::{Address, Env, String, contract, contractimpl};

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn __constructor(_env: &Env) {
        panic!("I can't be initialized")
    }

    pub fn hello(env: &Env) -> String {
        String::from_str(env, "hi, I'm a secret v3!")
    }

    pub fn custom_upgrade(env: &Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let _admin: Address = unsafe { admin_from_storage(env).unwrap_unchecked() };
        // admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}

fn admin_from_storage(env: &Env) -> Option<Address> {
    env.storage().instance().get(admin_sep::STORAGE_KEY)
}
