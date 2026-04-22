#![no_std]
#![allow(clippy::too_many_arguments)]
use admin_sep::{Administratable, Upgradable};
use soroban_sdk::Val;
use soroban_sdk::Vec;
use soroban_sdk::{assert_with_error, contract, contractimpl, Address, Env};

pub mod error;
pub mod events;
pub mod name;
pub mod registry;
pub(crate) mod storage;
pub mod version;

use crate::registry::contract::Proxyable;
pub use error::Error;
use registry::{
    contract::{Batchable, Deployable, Manageable, Redeployable},
    wasm::Publishable,
};
use storage::Storage;

#[contract]
pub struct Contract;

#[contractimpl(contracttrait)]
impl Administratable for Contract {}

#[contractimpl(contracttrait)]
impl Upgradable for Contract {}

#[contractimpl(contracttrait)]
impl Deployable for Contract {}

#[contractimpl(contracttrait)]
impl Redeployable for Contract {}

#[contractimpl(contracttrait)]
impl Batchable for Contract {}

#[contractimpl(contracttrait)]
impl Manageable for Contract {}

#[contractimpl(contracttrait)]
impl Publishable for Contract {}

#[contractimpl(contracttrait)]
impl Proxyable for Contract {}

#[contractimpl]
impl Contract {
    /// - `admin`: account which will: upgrade this Registry itself; add, set, or remove `manager`
    /// - `manager`: optional. If set, makes this a *managed* registry, meaning `publish`, `register_contract`, & `deploy` must be approved by the manager before caller's account is considered trusted for that contract/wasm name.
    /// - `root`: if None, this registry is the root registry — it has no namespace, other registries (like the `unverified` one) are registered in it, and the constructor auto-deploys the `unverified` registry. If Some, this is a subregistry that defers to the given root for resolving sibling subregistry names during cross-registry deploys.
    #[allow(clippy::needless_pass_by_value)]
    pub fn __constructor(
        env: &Env,
        admin: &Address,
        manager: Option<Address>,
        root: Option<Address>,
    ) -> Result<(), Error> {
        Self::set_admin(env, admin);
        if let Some(manager) = &manager {
            Storage::set_manager_no_auth(env, manager);
        }
        if let Some(root_address) = &root {
            Storage::new(env).root_registry.set(root_address);
        } else {
            assert_with_error!(env, manager.is_some(), Error::ManagerRequired);
            Self::deploy_unverified_and_claim_registry(env, admin)?;
        }
        Ok(())
    }

    /// The manager account which if set authorizes initial publishes and claiming a contract id
    pub fn manager(env: &Env) -> Option<Address> {
        Storage::manager(env)
    }

    /// Admin can set the new manager
    pub fn set_manager(env: &Env, new_manager: &Address) {
        Storage::set_manager(env, new_manager);
    }

    /// Admin can remove manager
    pub fn remove_manager(env: &Env) {
        Storage::remove_manager(env);
    }
}

#[cfg(test)]
mod test;
