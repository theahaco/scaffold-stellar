#![no_std]
use loam_subcontract_core::{admin::Admin, Core};

pub mod error;
pub mod subcontract;

pub use error::Error;
use subcontract::{Calculator, Token};

#[loam_sdk::derive_contract(Core(Admin), Token(Calculator))]
pub struct Contract;
