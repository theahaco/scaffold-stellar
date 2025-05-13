#![no_std]
use loam_sdk::soroban_sdk;
use loam_subcontract_core::{admin::Admin, Core};

use registry::{contract::C as Contract_, wasm::W, Deployable, Publishable, Redeployable};

pub mod error;
pub mod registry;
pub mod util;
pub mod version;

use error::Error;
use version::Version;

#[loam_sdk::derive_contract(
    Core(Admin),
    Publishable(W),
    Deployable(Contract_),
    Redeployable(Contract_)
)]
pub struct Contract;

#[cfg(test)]
mod test;
