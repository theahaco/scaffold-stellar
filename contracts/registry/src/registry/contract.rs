#![allow(non_upper_case_globals)]
use loam_sdk::{
    loamstorage,
    soroban_sdk::{
        self, assert_with_error, contracttype, env, symbol_short, to_string, Address, BytesN, Env,
        IntoVal, PersistentMap, String, Symbol,
    },
    vec,
};
use loam_subcontract_core::Core;

use crate::{
    error::Error,
    name::validate,
    registry::Publishable,
    util::{hash_string, REGISTRY},
    Contract as Contract_,
};

use super::{wasm::W, IsDeployable, IsRedeployable};

#[contracttype]
pub struct DeployEventData {
    wasm_name: String,
    contract_name: String,
    version: String,
    deployer: Address,
    contract_id: Address,
}

#[loamstorage]
pub struct C {
    pub r: PersistentMap<String, Address>,
}

impl C {
    fn upgrade(
        &self,
        name: &String,
        wasm_hash: &BytesN<32>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let contract_id = self.fetch_contract_id(name.clone())?;
        if let Ok(Ok(author)) = env().try_invoke_contract::<Address, Error>(
            &contract_id,
            &symbol_short!("admin"),
            vec![],
        ) {
            author.require_auth();
        }
        let fn_name = upgrade_fn.unwrap_or_else(|| symbol_short!("upgrade"));
        env().invoke_contract::<()>(&contract_id, &fn_name, vec![wasm_hash.into_val(env())]);
        Ok(contract_id)
    }
}

impl IsDeployable for C {
    fn deploy(
        &mut self,
        wasm_name: String,
        version: Option<String>,
        contract_name: String,
        admin: Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    ) -> Result<Address, Error> {
        validate(&contract_name)?;
        let env = env();
        if self.r.has(contract_name.clone()) {
            return Err(Error::AlreadyDeployed);
        }
        if contract_name == to_string(REGISTRY) {
            assert_with_error!(
                env,
                Contract_::admin_get().unwrap() == admin,
                Error::AdminOnly
            );
        }
        // signed by owner
        admin.require_auth();
        let hash = Contract_::fetch_hash(wasm_name.clone(), version.clone())?;
        let salt: BytesN<32> = hash_string(&contract_name).into();
        let address = deploy_and_init(salt, hash, init);
        self.r.set(contract_name.clone(), &address);

        // Publish a deploy event
        let version = version.map_or_else(|| W::default().most_recent_version(&wasm_name), Ok)?;
        let deploy_data = DeployEventData {
            wasm_name,
            contract_name,
            version,
            deployer: admin,
            contract_id: address.clone(),
        };
        env.events()
            .publish((symbol_short!("deploy"),), deploy_data);

        Ok(address)
    }

    fn fetch_contract_id(&self, contract_name: String) -> Result<Address, Error> {
        self.r
            .get(contract_name)
            .ok_or(Error::NoSuchContractDeployed)
    }
}

fn deploy_and_init(
    salt: impl IntoVal<Env, BytesN<32>>,
    wasm_hash: BytesN<32>,
    args: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
) -> Address {
    let deployer = env().deployer().with_current_contract(salt.into_val(env()));
    if let Some(args) = args {
        deployer.deploy_v2(wasm_hash, args)
    } else {
        deployer.deploy_v2(wasm_hash, ())
    }
}

impl IsRedeployable for C {
    fn dev_deploy(
        &mut self,
        name: soroban_sdk::String,
        wasm: soroban_sdk::Bytes,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error> {
        let wasm_hash = env().deployer().upload_contract_wasm(wasm);
        self.upgrade(&name, &wasm_hash, upgrade_fn)
    }

    fn upgrade_contract(
        &mut self,
        name: String,
        wasm_name: String,
        version: Option<String>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let wasm_hash = Contract_::fetch_hash(wasm_name.clone(), version.clone())?;
        self.upgrade(&name, &wasm_hash, upgrade_fn)
    }
}
