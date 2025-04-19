#![no_std]
use loam_sdk::derive_contract;
use loam_subcontract_core::{admin::Admin, Core};

#[derive_contract(Core(Admin))]
pub struct Contract;
