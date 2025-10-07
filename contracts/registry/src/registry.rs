use soroban_sdk::{contracttrait, Env};

use crate::error::Error;

pub mod contract;
pub mod wasm;
#[contracttrait]
pub trait IsPublishable {
    /// Fetch the hash of a Wasm binary from the registry
    fn fetch_hash(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
    ) -> Result<soroban_sdk::BytesN<32>, Error>;

    /// Most recent version of the published Wasm binary
    fn current_version(
        env: &Env,
        wasm_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::String, Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish(
        env: &Env,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm: soroban_sdk::Bytes,
        version: soroban_sdk::String,
    ) -> Result<(), Error>;

    /// Publish a binary. If contract had been previously published only previous author can publish again
    fn publish_hash(
        env: &Env,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        version: soroban_sdk::String,
    ) -> Result<(), Error>;
}

#[contracttrait]
pub trait IsDeployable {
    /// Deploys a new published contract returning the deployed contract's id.
    /// If no salt provided it will use the current sequence number.
    fn deploy(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        contract_name: soroban_sdk::String,
        admin: soroban_sdk::Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Look up the contract id of a deployed contract
    fn fetch_contract_id(
        env: &Env,
        contract_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::Address, Error>;
}

#[contracttrait]
pub trait IsRedeployable {
    /// Skips the publish step to deploy a contract directly, keeping the name
    fn dev_deploy(
        env: &Env,
        name: soroban_sdk::String,
        wasm: soroban_sdk::Bytes,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Upgrades a contract by calling the upgrade function.
    /// Default is 'upgrade' and expects that first arg is the corresponding wasm hash
    fn upgrade_contract(
        env: &Env,
        name: soroban_sdk::String,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error>;
}

