use loam_sdk::soroban_sdk::{self, Lazy};

use crate::{
    error::Error,
    version::{self, Version},
};

pub mod contract;
pub mod wasm;

pub use contract::Contract;
pub use wasm::Wasm;

#[loam_sdk::subcontract]
pub trait IsPublishable {
    /// Fetch the hash of a Wasm binary from the registry
    fn fetch_hash(
        &self,
        wasm_name: soroban_sdk::String,
        version: Option<Version>,
    ) -> Result<soroban_sdk::BytesN<32>, Error> {
        Ok(self.fetch(wasm_name, version)?.hash)
    }

    /// Most recent version of the published Wasm binary
    fn current_version(&self, wasm_name: soroban_sdk::String) -> Result<Version, Error>;

    /// Fetch details of the published binary
    fn fetch(
        &self,
        wasm_name: soroban_sdk::String,
        version: Option<Version>,
    ) -> Result<crate::metadata::PublishedWasm, Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish(
        &mut self,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm: soroban_sdk::Bytes,
        repo: Option<soroban_sdk::String>,
        kind: Option<version::Update>,
    ) -> Result<(), Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish_hash(
        &mut self,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        repo: Option<soroban_sdk::String>,
        kind: Option<version::Update>,
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
        salt: Option<soroban_sdk::BytesN<32>>,
        init: Option<(soroban_sdk::Symbol, soroban_sdk::Vec<soroban_sdk::Val>)>,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Fetch contract id
    fn fetch_contract_id(
        &self,
        contract_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::Address, Error>;
}

#[loam_sdk::subcontract]
pub trait IsDevDeployable {
    /// Skips the publish step to deploy a contract directly, keeping the name
    fn dev_deploy(
        &mut self,
        name: soroban_sdk::String,
        owner: soroban_sdk::Address,
        wasm: soroban_sdk::Bytes,
    ) -> Result<soroban_sdk::Address, Error>;
}
