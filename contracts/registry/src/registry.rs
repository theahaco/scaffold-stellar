use soroban_sdk::{contracttrait, Env};

use crate::error::Error;

pub mod contract;
pub mod wasm;
#[contracttrait]
pub trait Publishable {
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
pub trait Deployable {
    /// Deploys a new published contract returning the deployed contract's id
    /// and claims the contract name.
    /// If no salt provided it will use the current sequence number.
    /// If no deployer is provided it uses the contract as the deployer
    fn deploy(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        contract_name: soroban_sdk::String,
        admin: soroban_sdk::Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
        deployer: Option<soroban_sdk::Address>,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Claim name for an existing contract which wasn't deployed by the registry
    fn claim_contract_id(
        env: &Env,
        contract_name: soroban_sdk::String,
        contract_address: soroban_sdk::Address,
        owner: soroban_sdk::Address,
    ) -> Result<(), Error>;

    /// Look up the contract id of a deployed contract
    fn fetch_contract_id(
        env: &Env,
        contract_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Look up the owner of a deployed contract
    fn fetch_contract_owner(
        env: &Env,
        contract_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::Address, Error>;

    /// Deploys a new published contract returning the deployed contract's id
    /// but does not claim the contract name.
    /// If name is provided it used as the salt.
    /// Otherwise if no salt provided it will use a random one.
    fn deploy_without_claiming(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        contract_name: Option<soroban_sdk::String>,
        salt: Option<soroban_sdk::BytesN<32>>,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
        deployer: soroban_sdk::Address,
    ) -> Result<soroban_sdk::Address, Error>;
}

#[contracttrait]
pub trait Redeployable {
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
