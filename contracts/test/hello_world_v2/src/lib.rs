#![no_std]
use admin_sep::*;
use soroban_sdk::{Address, Env, String, contract, contractimpl};

#[contract]
pub struct Contract;

#[contractimpl(contracttrait)]
impl Administratable for Contract {}

#[contractimpl(contracttrait)]
impl Upgradable for Contract {}

#[contractimpl]
impl Contract {
    pub fn __constructor(env: &Env, admin: Address) {
        Self::set_admin(env, admin);
    }
    pub fn hello(env: &Env) -> String {
        String::from_str(env, "hi, I'm a v2!")
    }
}
