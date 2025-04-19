#![no_std]
// // Currently need to import `self` because `contracttype` expects it in the namespace
use loam_sdk::derive_contract;
use loam_subcontract_core::{admin::Admin, Core};

mod status_message;
pub use status_message::*;

#[derive_contract(Core(Admin), Postable(StatusMessage))]
pub struct Contract;
