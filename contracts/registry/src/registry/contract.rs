#![allow(non_upper_case_globals)]
use loam_sdk::{
    loamstorage,
    soroban_sdk::{
        self, contracttype, env, symbol_short, Address, BytesN, Env, IntoVal, PersistentMap,
        String, Symbol,
    },
    vec,
};

use crate::{
    error::Error, registry::Publishable, util::hash_string, version::Version, Contract as Contract_,
};

use super::{wasm::Wasm, IsDeployable, IsRedeployable};

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

impl Contract {
    fn redeploy(
        &self,
        name: &String,
        wasm_hash: &BytesN<32>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let contract_id = self.fetch_contract_id(name.clone())?;
        let fn_name = upgrade_fn.unwrap_or_else(|| symbol_short!("redeploy"));
        env().invoke_contract::<()>(&contract_id, &fn_name, vec![wasm_hash.into_val(env())]);
        Ok(contract_id)
    }
}

impl IsDeployable for Contract {
    fn deploy(
        &mut self,
        wasm_name: String,
        version: Option<Version>,
        contract_name: String,
        owner: Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    ) -> Result<Address, Error> {
        let env = env();
        if self.registry.has(contract_name.clone()) {
            return Err(Error::AlreadyDeployed);
        }
        // signed by owner
        owner.require_auth();
        let hash = Contract_::fetch_hash(wasm_name.clone(), version.clone())?;
        let salt: BytesN<32> = hash_string(&contract_name).into();
        let address = deploy_and_init(salt, hash, init);
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
    salt: impl IntoVal<Env, BytesN<32>>,
    wasm_hash: BytesN<32>,
    args: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
) -> Address {
    // Deploy the contract using the installed Wasm code with given hash.
    env()
        .deployer()
        .with_current_contract(salt.into_val(env()))
        .deploy_v2(wasm_hash, args.unwrap_or_else(|| vec![]))
}

impl IsRedeployable for Contract {
    fn dev_deploy(
        &mut self,
        name: soroban_sdk::String,
        wasm: soroban_sdk::Bytes,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error> {
        let wasm_hash = env().deployer().upload_contract_wasm(wasm);
        self.redeploy(&name, &wasm_hash, upgrade_fn)
    }

    fn upgrade_contract(
        &mut self,
        name: String,
        wasm_name: String,
        version: Option<Version>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let wasm_hash = Contract_::fetch_hash(wasm_name.clone(), version.clone())?;
        self.redeploy(&name, &wasm_hash, upgrade_fn)
    }
}
