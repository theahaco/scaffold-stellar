#![allow(non_upper_case_globals)]
use loam_sdk::{
    loamstorage,
    soroban_sdk::{
        self, assert_with_error, contracttype, env, symbol_short, to_string, Address, BytesN, Env,
        IntoVal, InvokeError, PersistentMap, String, Symbol,
    },
    vec,
};
use loam_subcontract_core::Core;

use crate::{
    error::Error,
    name::canonicalize,
    util::{hash_string, MAX_BUMP, REGISTRY},
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
    fn get(&self, contract_name: &String) -> Result<Address, Error> {
        self.r
            .get(contract_name.clone())
            .ok_or(Error::NoSuchContractDeployed)
    }

    fn upgrade(
        &self,
        name: &String,
        wasm_hash: &BytesN<32>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let name = canonicalize(name)?;
        let contract_id = self.get(&name)?;
        self.r.extend_ttl(name, MAX_BUMP, MAX_BUMP);
        if let Ok(Ok(author)) = env().try_invoke_contract::<Address, Error>(
            &contract_id,
            &symbol_short!("admin"),
            vec![],
        ) {
            author.require_auth();
        }
        let fn_name = upgrade_fn.unwrap_or_else(|| symbol_short!("upgrade"));
        let _ = env()
            .try_invoke_contract::<(), InvokeError>(
                &contract_id,
                &fn_name,
                vec![wasm_hash.into_val(env())],
            )
            .map_err(|_| Error::UpgradeInvokeFailed)?;
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
        let contract_name = canonicalize(&contract_name)?;
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
        // signed by admin
        admin.require_auth();

        let mut wasm_registry = W::default();
        let hash = wasm_registry.get_hash_and_bump(&wasm_name, version.clone())?;
        let salt: BytesN<32> = hash_string(&contract_name).into();
        let contract_id = deploy_and_init(salt, hash, init);

        self.r.set(contract_name.clone(), &contract_id);

        let version = wasm_registry.get_version(&wasm_name, version)?;
        // Publish a deploy event
        let deploy_data = DeployEventData {
            wasm_name,
            contract_name,
            version,
            deployer: admin,
            contract_id: contract_id.clone(),
        };
        env.events()
            .publish((symbol_short!("deploy"),), deploy_data);
        Ok(contract_id)
    }

    fn fetch_contract_id(&self, contract_name: String) -> Result<Address, Error> {
        let contract_name = canonicalize(&contract_name)?;
        self.get(&contract_name)
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
        let wasm_hash = W::default().get_hash_and_bump(&wasm_name, version)?;
        self.upgrade(&name, &wasm_hash, upgrade_fn)
    }
}
