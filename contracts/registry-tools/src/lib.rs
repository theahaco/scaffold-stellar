#![no_std]
// use loam_subcontract_core::{admin::Admin, Core};

// use registry::{contract::C as Contract_, wasm::W as Wasm, Deployable, Publishable, Redeployable};
use version::{ProperVersion, Checker};

// pub mod error;
// pub mod name;
// pub mod registry;
// pub mod util;
pub mod version;

#[cfg(target_family = "wasm")]
mod alloc;

// pub use error::Error;

#[loam_sdk::derive_contract(
    ProperVersion(Checker)
)]
pub struct Contract;

// #[cfg(test)]
// mod test;
