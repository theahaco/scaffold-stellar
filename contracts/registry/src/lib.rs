#![no_std]

use admin_sep::{Administratable, Upgradable};
use soroban_sdk::{contract, contractimpl, Address, Env};

pub mod error;
pub mod events;
pub mod name;
pub mod registry;
mod util;
pub mod version;

mod storage;

pub use error::Error;

use crate::storage::Storage;

#[contract]
pub struct Contract;

#[contractimpl(contracttrait)]
impl Administratable for Contract {}

#[contractimpl(contracttrait)]
impl Upgradable for Contract {}

#[contractimpl]
impl Contract {
    /// Admin account authorizes: upgrade, adding, setting, or removing a manager.
    /// If contract has a manager account it must authorize initial publishes or claims or deploys
    pub fn __constructor(env: &Env, admin: Address, manager: Option<Address>) {
        Self::set_admin(env, admin);
        if let Some(manager) = manager {
            Storage::set_manager_no_auth(env, &manager);
        }
    }

    /// The manager account which if set authorizes initial publishes and claiming an contract id
    pub fn manager(env: &Env) -> Option<Address> {
        Storage::manager(env)
    }

    /// Admin can set the new manager
    pub fn set_manager(env: &Env, new_manager: Address) {
        Storage::set_manager(env, &new_manager);
    }

    /// Admin can remove manager
    pub fn remove_manager(env: &Env) {
        Storage::remove_manager(env);
    }
}

#[cfg(test)]
mod test;
