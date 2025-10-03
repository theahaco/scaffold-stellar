#![no_std]

use admin_sep::Administratable;
use soroban_sdk::{contract, contractimpl, Address, Env};

pub mod error;
pub mod name;
pub mod registry;
mod util;
pub mod version;

#[cfg(target_family = "wasm")]
mod alloc;
mod storage;

pub use error::Error;

#[contract]
pub struct Contract;

#[contractimpl]
impl Administratable for Contract {}

#[contractimpl]
impl Contract {
    pub fn __constructor(env: &Env, admin: Address) {
        Self::set_admin(env, admin);
    }
}

#[cfg(test)]
mod test;
