#![allow(non_upper_case_globals)]
use loam_sdk::{
    loamstorage,
    soroban_sdk::{
        self, contracttype, env, symbol_short, Address, BytesN, Env, IntoVal, PersistentMap,
        String, Symbol, Val,
    },
};

use crate::{
    error::Error, registry::Publishable, util::hash_string, version::Version, Contract as Contract_,
};

use super::{wasm::Wasm, IsDeployable, IsDevDeployable};

loam_sdk::import_contract!(example_core);

// Is the same as

// mod example_core {
//     use loam_sdk::soroban_sdk;
//     loam_sdk::soroban_sdk::contractimport!(file = "../../target/loam/example_core.wasm",);
// }

#[contracttype]
pub struct DeployEventData {
    wasm_name: String,
    contract_name: String,
    version: Version,
    deployer: Address,
    contract_id: Address,
}

#[loamstorage]
pub struct Contract {
    pub registry: PersistentMap<String, Address>,
}

impl IsDeployable for Contract {
    fn deploy(
        &mut self,
        wasm_name: String,
        version: Option<Version>,
        contract_name: String,
        owner: Address,
        salt: Option<BytesN<32>>,
        init: Option<(Symbol, soroban_sdk::Vec<soroban_sdk::Val>)>,
    ) -> Result<Address, Error> {
        let env = env();
        if self.registry.has(contract_name.clone()) {
            return Err(Error::NoSuchContractDeployed);
        }
        // signed by owner
        owner.require_auth();
        let hash = Contract_::fetch_hash(wasm_name.clone(), version.clone())?;
        let salt: BytesN<32> = salt.unwrap_or_else(|| hash_string(&contract_name).into_val(env));
        let address = deploy_and_init(&owner, salt, hash)?;
        if let Some((init_fn, args)) = init {
            let _ = env.invoke_contract::<Val>(&address, &init_fn, args);
        }
        self.registry.set(contract_name.clone(), &address);

        // Publish a deploy event
        let version =
            version.map_or_else(|| Wasm::default().most_recent_version(&wasm_name), Ok)?;
        let deploy_datas = DeployEventData {
            wasm_name,
            contract_name,
            version,
            deployer: owner,
            contract_id: address.clone(),
        };
        env.events()
            .publish((symbol_short!("deploy"),), deploy_datas);

        Ok(address)
    }

    fn fetch_contract_id(&self, contract_name: String) -> Result<Address, Error> {
        self.registry
            .get(contract_name)
            .ok_or(Error::NoSuchContractDeployed)
    }
}

fn deploy_and_init(
    owner: &Address,
    salt: impl IntoVal<Env, BytesN<32>>,
    wasm_hash: BytesN<32>,
) -> Result<Address, Error> {
    // Deploy the contract using the installed Wasm code with given hash.
    let address = env()
        .deployer()
        .with_current_contract(salt.into_val(env()))
        .deploy_v2(wasm_hash, ());
    // Set the owner of the contract to the given owner.
    let _ = example_core::Client::new(env(), &address)
        .try_admin_set(owner)
        .map_err(|_| Error::InitFailed)?;
    Ok(address)
}

impl IsDevDeployable for Contract {
    fn dev_deploy(
        &mut self,
        name: soroban_sdk::String,
        owner: soroban_sdk::Address,
        wasm: soroban_sdk::Bytes,
    ) -> Result<soroban_sdk::Address, Error> {
        let wasm_hash = env().deployer().upload_contract_wasm(wasm);
        if let Some(address) = self.registry.get(name.clone()) {
            let contract = example_core::Client::new(env(), &address);
            contract.redeploy(&wasm_hash);
            return Ok(address.clone());
        }
        let salt = hash_string(&name);
        let id = deploy_and_init(&owner, salt, wasm_hash)?;
        self.registry.set(name, &id);
        Ok(id)
    }
}
