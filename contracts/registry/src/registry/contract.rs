#![allow(non_upper_case_globals)]
use crate::name;
use crate::name::NormalizedName;
use crate::storage::ContractEntry;
use crate::storage::Storage;
use crate::ContractArgs;
use crate::ContractClient;
use admin_sep::{Administratable, AdministratableExtension};
use soroban_sdk::Val;
use soroban_sdk::Vec;
use soroban_sdk::{
    self, contractimpl, symbol_short, vec, Address, BytesN, Env, IntoVal, InvokeError, String,
    Symbol,
};

use crate::{
    error::Error,
    name::canonicalize,
    util::{hash_string, MAX_BUMP},
    Contract,
};

use super::{Deployable, Redeployable};

impl Contract {
    fn assert_no_contract_entry(
        env: &Env,
        contract_admin: &Address,
        contract_name: &NormalizedName,
    ) -> Result<(), Error> {
        if contract_name == &name::registry(env) {
            if &Self::admin(env) != contract_admin {
                return Err(Error::AdminOnly);
            }
        } else {
            // Currently require admin for deploying
            Self::require_admin(env);
        }
        Storage::new(env)
            .contract
            .get(contract_name)
            .is_none()
            .then_some(())
            .ok_or(Error::AlreadyDeployed)
    }

    fn get_contract_entry(env: &Env, contract_name: &NormalizedName) -> Result<ContractEntry, Error> {
        Storage::new(env)
            .contract
            .get(contract_name)
            .map(Into::into)
            .ok_or(Error::NoSuchContractDeployed)
    }

    fn get_contract_id(env: &Env, contract_name: &NormalizedName) -> Result<Address, Error> {
        Ok(Self::get_contract_entry(env, contract_name)?.contract)
    }

    fn get_contract_owner(env: &Env, contract_name: &NormalizedName) -> Result<Address, Error> {
        Ok(Self::get_contract_entry(env, contract_name)?.owner)
    }

    fn upgrade(
        env: &Env,
        name: &NormalizedName,
        wasm_hash: &BytesN<32>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let contract_id = Self::get_contract_id(env, &name)?;
        Storage::new(env)
            .contract
            .extend_ttl(&name, MAX_BUMP, MAX_BUMP);
        /*
        Here we check if the contract being upgrade supports the admin interface.
        If so we can fetch the admin and call require auth at the top level.
        This prevents needing to use a more complex auth entry because the signer must be authorized in
        the root of the call. However, if the contract doesn't implement the interface, then it is up to
        the caller to have constructed the auth entries correctly and the contract itself for handling how
        authorization is set up for upgrading.
         */
        if let Ok(Ok(author)) = env.try_invoke_contract::<Address, Error>(
            &contract_id,
            &symbol_short!("admin"),
            vec![&env],
        ) {
            author.require_auth();
        }
        let fn_name = upgrade_fn.unwrap_or_else(|| symbol_short!("upgrade"));
        let val = wasm_hash.into_val(env);
        let r = env.try_invoke_contract::<(), InvokeError>(&contract_id, &fn_name, vec![&env, val]);
        let _ = r.map_err(|_| Error::UpgradeInvokeFailed)?;
        Ok(contract_id)
    }

    fn claim_contract_name(
        env: &Env,
        contract_name: &NormalizedName,
        contract_id: &Address,
        contract_admin: &Address,
    ) -> Result<(), Error> {
        let mut contract_map = Storage::new(env).contract;
        contract_map.set(
            contract_name,
            &(contract_admin.clone(), contract_id.clone()),
        );
        crate::events::Claim {
            contract_name: contract_name.to_string(),
            contract_id: contract_id.clone(),
        }
        .publish(env);
        Ok(())
    }

    fn fetch_hash_and_deploy(
        env: &Env,
        wasm_name: &String,
        version: Option<String>,
        salt: BytesN<32>,
        init: Option<Vec<Val>>,
        deployer: Address,
    ) -> Result<Address, Error> {
        let wasm_name = wasm_name.try_into()?;
        let hash = Self::get_hash_and_bump(env, &wasm_name, version.clone())?;
        let contract_id = deploy_and_init(env, salt, hash, init, deployer.clone());
        let version = Self::get_version(env, &wasm_name, version)?;
        // Publish a deploy event
        crate::events::Deploy {
            wasm_name: wasm_name.to_string(),
            version,
            deployer,
            contract_id: contract_id.clone(),
        }
        .publish(env);
        Ok(contract_id)
    }
}

#[contractimpl]
impl Deployable for Contract {
    fn deploy(
        env: &Env,
        wasm_name: String,
        version: Option<String>,
        contract_name: String,
        admin: Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
        deployer: Option<Address>,
    ) -> Result<Address, Error> {
        let contract_name = contract_name.try_into()?;
        // signed by admin of contract
        Self::assert_no_contract_entry(env, &admin, &contract_name)?;
        admin.require_auth();
        let deployer = deployer.unwrap_or_else(|| env.current_contract_address());
        let salt: BytesN<32> = hash_string(env, contract_name.as_string()).into();
        let contract_id = Self::fetch_hash_and_deploy(
            env,
            &wasm_name,
            version.clone(),
            salt,
            init,
            deployer.clone(),
        )?;
        Self::claim_contract_name(env, &contract_name, &contract_id, &admin)?;
        Ok(contract_id)
    }
    fn deploy_without_claiming(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        contract_name: Option<soroban_sdk::String>,
        salt: Option<soroban_sdk::BytesN<32>>,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
        deployer: soroban_sdk::Address,
    ) -> Result<Address, Error> {
        let contract_name = contract_name.as_ref().map(canonicalize).transpose()?;
        deployer.require_auth();
        let salt: BytesN<32> = contract_name
            .as_ref()
            .map(|name| hash_string(env, name).into())
            .or(salt)
            .unwrap_or_else(|| env.prng().gen());
        Self::fetch_hash_and_deploy(env, &wasm_name, version.clone(), salt, init, deployer)
    }
    fn claim_contract_id(
        env: &Env,
        contract_name: String,
        contract_address: Address,
        owner: Address,
    ) -> Result<(), Error> {
        let contract_name = contract_name.try_into()?;
        owner.require_auth();
        Self::assert_no_contract_entry(env, &owner, &contract_name)?;
        Self::claim_contract_name(env, &contract_name, &contract_address, &owner)
    }

    fn fetch_contract_id(env: &Env, contract_name: String) -> Result<Address, Error> {
        Self::get_contract_id(env, &contract_name.try_into()?)
    }
    fn fetch_contract_owner(env: &Env, contract_name: String) -> Result<Address, Error> {
        Self::get_contract_owner(env, &contract_name.try_into()?)
    }
}

fn deploy_and_init(
    env: &Env,
    salt: impl IntoVal<Env, BytesN<32>>,
    wasm_hash: BytesN<32>,
    args: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    deployer: Address,
) -> Address {
    let deployer = env.deployer().with_address(deployer, salt.into_val(env));
    if let Some(args) = args {
        deployer.deploy_v2(wasm_hash, args)
    } else {
        deployer.deploy_v2(wasm_hash, ())
    }
}

#[contractimpl]
impl Redeployable for Contract {
    fn dev_deploy(
        env: &Env,
        name: soroban_sdk::String,
        wasm: soroban_sdk::Bytes,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error> {
        let wasm_hash = env.deployer().upload_contract_wasm(wasm);
        Self::upgrade(env, &name.try_into()?, &wasm_hash, upgrade_fn)
    }

    fn upgrade_contract(
        env: &Env,
        name: String,
        wasm_name: String,
        version: Option<String>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let wasm_hash = Self::get_hash_and_bump(env, &wasm_name.try_into()?, version)?;
        Self::upgrade(env, &name.try_into()?, &wasm_hash, upgrade_fn)
    }
}
