#![no_std]
use admin_sep::{Administratable, Upgradable};
use soroban_sdk::{Address, Env, String, contract, contractimpl};

#[contract]
pub struct Contract;

#[contractimpl]
impl Administratable for Contract {}

#[contractimpl]
impl Upgradable for Contract {}

#[contractimpl]
impl Contract {
    pub fn __constructor(env: &Env, admin: &Address) {
        Self::set_admin(env, admin);
    }
    pub fn hello(_: &Env, to: String) -> String {
        to
    }
}
