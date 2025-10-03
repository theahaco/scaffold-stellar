#![no_std]
use loam_subcontract_core::{Core, admin::Admin};

use registry::{Deployable, Publishable, Redeployable, contract::C as Contract_, wasm::W as Wasm};

pub mod error;
pub mod name;
pub mod registry;
mod util;
pub mod version;

#[cfg(target_family = "wasm")]
mod alloc;

pub use error::Error;

#[loam_sdk::derive_contract(
    Core(Admin),
    Publishable(Wasm),
    Deployable(Contract_),
    Redeployable(Contract_)
)]
pub struct Contract;

#[cfg(test)]
mod test;
