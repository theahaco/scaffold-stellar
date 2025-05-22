#![no_std]
use loam_subcontract_core::{admin::Admin, Core};

use registry::{contract::C as Contract_, wasm::W as Wasm, Deployable, Publishable, Redeployable};

pub mod error;
pub mod name;
pub mod registry;
pub mod util;
pub mod version;

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
