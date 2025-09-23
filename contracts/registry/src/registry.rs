use loam_sdk::soroban_sdk::Lazy;

use crate::error::Error;

pub mod contract;
pub mod wasm;

pub use contract::C;
pub use wasm::W;

#[loam_sdk::subcontract]
pub trait IsPublishable {
    /// Fetch the hash of a Wasm binary from the registry
    fn fetch_hash(
        &self,
        wasm_name: loam_sdk::soroban_sdk::String,
        version: Option<loam_sdk::soroban_sdk::String>,
    ) -> Result<loam_sdk::soroban_sdk::BytesN<32>, Error>;

    /// Most recent version of the published Wasm binary
    fn current_version(
        &self,
        wasm_name: loam_sdk::soroban_sdk::String,
    ) -> Result<loam_sdk::soroban_sdk::String, Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish(
        &mut self,
        wasm_name: loam_sdk::soroban_sdk::String,
        author: loam_sdk::soroban_sdk::Address,
        wasm: loam_sdk::soroban_sdk::Bytes,
        version: loam_sdk::soroban_sdk::String,
    ) -> Result<(), Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish_hash(
        &mut self,
        wasm_name: loam_sdk::soroban_sdk::String,
        author: loam_sdk::soroban_sdk::Address,
        wasm_hash: loam_sdk::soroban_sdk::BytesN<32>,
        version: loam_sdk::soroban_sdk::String,
    ) -> Result<(), Error>;

    fn keys(
        &self,
        wasm_name: loam_sdk::soroban_sdk::String,
    ) -> Result<loam_sdk::soroban_sdk::Vec<loam_sdk::soroban_sdk::String>, Error>;
}

#[loam_sdk::subcontract]
pub trait IsDeployable {
    /// Deploys a new published contract returning the deployed contract's id.
    /// If no salt provided it will use the current sequence number.
    fn deploy(
        &mut self,
        wasm_name: loam_sdk::soroban_sdk::String,
        version: Option<loam_sdk::soroban_sdk::String>,
        contract_name: loam_sdk::soroban_sdk::String,
        admin: loam_sdk::soroban_sdk::Address,
        init: Option<loam_sdk::soroban_sdk::Vec<loam_sdk::soroban_sdk::Val>>,
    ) -> Result<loam_sdk::soroban_sdk::Address, Error>;

    /// Look up the contract id of a deployed contract
    fn fetch_contract_id(
        &self,
        contract_name: loam_sdk::soroban_sdk::String,
    ) -> Result<loam_sdk::soroban_sdk::Address, Error>;
}

#[loam_sdk::subcontract]
pub trait IsRedeployable {
    /// Skips the publish step to deploy a contract directly, keeping the name
    fn dev_deploy(
        &mut self,
        name: loam_sdk::soroban_sdk::String,
        wasm: loam_sdk::soroban_sdk::Bytes,
        upgrade_fn: Option<loam_sdk::soroban_sdk::Symbol>,
    ) -> Result<loam_sdk::soroban_sdk::Address, Error>;

    /// Upgrades a contract by calling the upgrade function.
    /// Default is 'upgrade' and expects that first arg is the corresponding wasm hash
    fn upgrade_contract(
        &mut self,
        name: loam_sdk::soroban_sdk::String,
        wasm_name: loam_sdk::soroban_sdk::String,
        version: Option<loam_sdk::soroban_sdk::String>,
        upgrade_fn: Option<loam_sdk::soroban_sdk::Symbol>,
    ) -> Result<loam_sdk::soroban_sdk::Address, Error>;
}
