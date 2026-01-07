#![no_std]

use admin_sep::{Administratable, Upgradable};
use soroban_sdk::{contract, contractimpl, vec, Address, Env, Executable, IntoVal, String, Val};

pub mod error;
pub mod events;
pub mod name;
pub mod registry;
mod util;
pub mod version;

mod storage;

pub use error::Error;
use storage::Storage;

use crate::{name::NormalizedName, registry::contract::deploy_and_init};

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
    pub fn __constructor(env: &Env, admin: Address, manager: Option<Address>, is_verified: bool) {
        Self::set_admin(env, admin.clone());
        if let Some(manager) = manager.as_ref() {
            Storage::set_manager_no_auth(env, manager);
        }

        if is_verified && manager.is_some() {
            unsafe {
                if let Executable::Wasm(wasm_hash) = env
                    .current_contract_address()
                    .executable()
                    .unwrap_unchecked()
                {
                    let contract_name =
                        NormalizedName::new_unchecked(String::from_str(env, "unverified"));
                    let args = vec![
                        env,
                        admin.as_val().clone(),
                        Val::from_void().into(),
                        false.into_val(env),
                    ];
                    let contract_address = deploy_and_init(
                        env,
                        contract_name.hash(),
                        wasm_hash,
                        Some(args),
                        env.current_contract_address(),
                    );
                    Self::claim_contract_name(env, &contract_name, &contract_address, &admin);
                }
            }
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
