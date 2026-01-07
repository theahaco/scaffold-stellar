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
    /// - `admin`: account which will: upgrade this Registry itself; add, set, or remove `manager`
    /// - `manager`: optional. If set, makes this a *managed* registry, meaning `publish`, `register_contract`, & `deploy` must be approved by the manager before caller's account is considered trusted for that contract/wasm name.
    /// - `is_root`: if true, this registry is the root registry, meaning it has no namespace. Other Registry contracts, like the `unverified` one, are themselves registered in the root Registry. If `is_root` is true, this constructor will also auto-deploy the `unverified` Registry.
    pub fn __constructor(env: &Env, admin: Address, manager: Option<Address>, is_root: bool) {
        Self::set_admin(env, admin.clone());
        if let Some(manager) = manager.as_ref() {
            Storage::set_manager_no_auth(env, manager);
        }
        if is_root && manager.is_some() {
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
                    Self::register_contract_name(env, &contract_name, &contract_address, &admin);
                }
            }
        }
    }

    /// The manager account which if set authorizes initial publishes and claiming a contract id
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
