use loam_sdk::soroban_sdk::{self, Lazy};

use crate::{
    error::Error,
    version::{self, Version},
};

pub mod contract;
pub mod wasm;

pub use contract::C;
pub use wasm::W;

#[loam_sdk::subcontract]
pub trait IsPublishable {
    /// Fetch the hash of a Wasm binary from the registry
    fn fetch_hash(
        &self,
        wasm_name: soroban_sdk::String,
        version: Option<version::Version>,
    ) -> Result<soroban_sdk::BytesN<32>, Error>;

    /// Most recent version of the published Wasm binary
    fn current_version(&self, wasm_name: soroban_sdk::String) -> Result<Version, Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish(
        &mut self,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm: soroban_sdk::Bytes,
        version: version::Version,
    ) -> Result<(), Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish_hash(
        &mut self,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        version: version::Version,
    ) -> Result<(), Error>;
}

#[loam_sdk::subcontract]
pub trait IsDeployable {
    /// Deploys a new published contract returning the deployed contract's id.
    /// If no salt provided it will use the current sequence number.
    fn deploy(
        &mut self,
        wasm_name: soroban_sdk::String,
        version: Option<Version>,
        contract_name: soroban_sdk::String,
        admin: soroban_sdk::Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Look up the contract id of a deployed contract
    fn fetch_contract_id(
        &self,
        contract_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::Address, Error>;
}

#[loam_sdk::subcontract]
pub trait IsRedeployable {
    /// Skips the publish step to deploy a contract directly, keeping the name
    fn dev_deploy(
        &mut self,
        name: soroban_sdk::String,
        wasm: soroban_sdk::Bytes,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Upgrades a contract by calling the upgrade function.
    /// Default is 'redeploy' and expects that first arg is the corresponding wasm hash
    fn upgrade_contract(
        &mut self,
        name: soroban_sdk::String,
        wasm_name: soroban_sdk::String,
        version: Option<version::Version>,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error>;
}
