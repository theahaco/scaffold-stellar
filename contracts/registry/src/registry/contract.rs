#![allow(non_upper_case_globals)]
use crate::name;
use crate::name::NormalizedName;
use crate::storage::ContractEntry;
use crate::storage::Storage;

use soroban_sdk::contracttrait;
use soroban_sdk::Executable;
use soroban_sdk::Val;
use soroban_sdk::Vec;
use soroban_sdk::{
    self, symbol_short, vec, Address, BytesN, Env, IntoVal, InvokeError, String, Symbol,
};

use crate::{error::Error, Contract};

impl Contract {
    pub(crate) fn assert_no_contract_entry_and_authorize(
        env: &Env,
        contract_admin: &Address,
        contract_name: &NormalizedName,
    ) -> Result<(), Error> {
        if let Some(manager) = Storage::manager(env) {
            // Currently require admin for deploying
            manager.require_auth();
        } else {
            contract_admin.require_auth();
        }
        let is_available = !Storage::new(env).contract.has(contract_name);
        is_available.then_some(()).ok_or(Error::AlreadyDeployed)
    }

    fn get_contract_entry(
        env: &Env,
        contract_name: &NormalizedName,
    ) -> Result<ContractEntry, Error> {
        Storage::new(env)
            .contract
            .get(contract_name)
            .ok_or(Error::NoSuchContractDeployed)
    }

    pub(crate) fn get_contract_id(
        env: &Env,
        contract_name: &NormalizedName,
    ) -> Result<Address, Error> {
        Ok(Self::get_contract_entry(env, contract_name)?.contract)
    }

    pub(crate) fn get_contract_owner(
        env: &Env,
        contract_name: &NormalizedName,
    ) -> Result<Address, Error> {
        Ok(Self::get_contract_entry(env, contract_name)?.owner)
    }

    pub(crate) fn upgrade_internal(
        env: &Env,
        name: &NormalizedName,
        wasm_hash: &BytesN<32>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let contract_id = Self::get_contract_id(env, name)?;
        Storage::new(env).contract.extend_ttl_max(name);
        /*
        Here we check if the contract being upgrade supports the admin interface.
        If so we can fetch the admin and call require auth at the top level.
        This prevents needing to use a more complex auth entry because the signer must be authorized in
        the root of the call. However, if the contract doesn't implement the interface, then it is up to
        the caller to have constructed the auth entries correctly and the contract itself for handling how
        authorization is set up for upgrading.
         */
        if let Ok(Ok(admin)) = env.try_invoke_contract::<Address, Error>(
            &contract_id,
            &symbol_short!("admin"),
            vec![&env],
        ) {
            admin.require_auth();
        }
        let fn_name = upgrade_fn.unwrap_or_else(|| symbol_short!("upgrade"));
        let val = wasm_hash.into_val(env);
        let r = env.try_invoke_contract::<(), InvokeError>(&contract_id, &fn_name, vec![&env, val]);
        let _ = r.map_err(|_| Error::UpgradeInvokeFailed)?;
        Ok(contract_id)
    }

    pub(crate) fn register_contract_name(
        env: &Env,
        contract_name: &NormalizedName,
        contract_id: &Address,
        contract_admin: &Address,
    ) {
        let mut contract_map = Storage::new(env).contract;
        contract_map.set(
            contract_name,
            &ContractEntry {
                owner: contract_admin.clone(),
                contract: contract_id.clone(),
            },
        );
        crate::events::Register {
            contract_name: contract_name.to_string(),
            contract_id: contract_id.clone(),
        }
        .publish(env);
    }

    pub(crate) fn fetch_hash_and_deploy(
        env: &Env,
        wasm_name: NormalizedName,
        version: Option<String>,
        salt: impl IntoVal<Env, BytesN<32>>,
        init: Option<Vec<Val>>,
        deployer: Address,
    ) -> Result<Address, Error> {
        let hash = Self::get_hash_and_bump(env, &wasm_name, version.clone())?;
        let contract_id = deploy_and_init(env, salt, hash, init, deployer.clone());
        let version = Self::get_version(env, &wasm_name, version)?;
        crate::events::Deploy {
            wasm_name: wasm_name.to_string(),
            version,
            deployer,
            contract_id: contract_id.clone(),
        }
        .publish(env);
        Ok(contract_id)
    }

    /// This method is used in the constructor when the contract is a root registry.
    /// It deploys an unverified contract and registers it with the name `unverified`
    /// It then registers the current contract with `register`.
    ///
    /// # Unsafe
    /// To deploy the unverified contract we need to fetch the hash of this contract.
    /// Since we know that this contract is an executable we can skip checking when unwrapping
    /// which is unsafe.
    ///
    /// Furthermore it uses the NormalizedName::new_unchecked, which is unsafe because it skips validating
    /// the name, which we know already to be valid.
    pub(crate) fn deploy_unverified_and_claim_registry(env: &Env, admin: &Address) {
        unsafe {
            if let Executable::Wasm(wasm_hash) = env
                .current_contract_address()
                .executable()
                .unwrap_unchecked()
            {
                let contract_name =
                    NormalizedName::new_unchecked(String::from_str(env, "unverified"));
                let args = vec![
                    env,
                    *admin.as_val(),
                    Val::from_void().into(),
                    false.into_val(env),
                ];
                let contract_address = deploy_and_init(
                    env,
                    contract_name.hash(),
                    wasm_hash,
                    Some(args),
                    env.current_contract_address(),
                );
                Self::register_contract_name(env, &contract_name, &contract_address, admin);
                Self::register_contract_name(
                    env,
                    &name::registry(env),
                    &env.current_contract_address(),
                    admin,
                );
            }
        }
    }
}

pub(crate) fn deploy_and_init(
    env: &Env,
    salt: impl IntoVal<Env, BytesN<32>>,
    wasm_hash: BytesN<32>,
    args: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    deployer: Address,
) -> Address {
    let deployer = env.deployer().with_address(deployer, salt);
    if let Some(args) = args {
        deployer.deploy_v2(wasm_hash, args)
    } else {
        deployer.deploy_v2(wasm_hash, ())
    }
}

#[contracttrait]
pub trait Deployable {
    /// Deploys a new published contract returning the deployed contract's id
    /// and register the contract name.
    /// If no salt provided it will use the current sequence number.
    /// If no deployer is provided it uses the contract as the deployer
    /// Note: `deployer` is an advanced feature.
    /// If you need to resolve contract IDs deterministically without RPC calls,
    /// you can set a known Deployer account, which will be used as the `--salt`.
    fn deploy(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        contract_name: soroban_sdk::String,
        admin: soroban_sdk::Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
        deployer: Option<soroban_sdk::Address>,
    ) -> Result<soroban_sdk::Address, Error> {
        let contract_name = contract_name.try_into()?;
        Contract::assert_no_contract_entry_and_authorize(env, &admin, &contract_name)?;
        let deployer = deployer.unwrap_or_else(|| env.current_contract_address());
        let salt = contract_name.hash();
        let contract_id = Contract::fetch_hash_and_deploy(
            env,
            wasm_name.try_into()?,
            version.clone(),
            salt,
            init,
            deployer.clone(),
        )?;
        Contract::register_contract_name(env, &contract_name, &contract_id, &admin);
        Ok(contract_id)
    }

    /// Deploys a new published contract returning the deployed contract's id
    /// but does not register the contract name.
    /// Otherwise if no salt provided it will use a random one.
    fn deploy_unnamed(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
        salt: soroban_sdk::BytesN<32>,
        deployer: soroban_sdk::Address,
    ) -> Result<soroban_sdk::Address, Error> {
        deployer.require_auth();
        Contract::fetch_hash_and_deploy(env, wasm_name.try_into()?, version, salt, init, deployer)
    }

    /// Register a name for an existing contract which wasn't deployed by the registry
    fn register_contract(
        env: &Env,
        contract_name: soroban_sdk::String,
        contract_address: soroban_sdk::Address,
        owner: soroban_sdk::Address,
    ) -> Result<(), Error> {
        let contract_name = contract_name.try_into()?;
        Contract::assert_no_contract_entry_and_authorize(env, &owner, &contract_name)?;
        Contract::register_contract_name(env, &contract_name, &contract_address, &owner);
        Ok(())
    }

    /// Look up the contract id of a deployed contract
    fn fetch_contract_id(
        env: &Env,
        contract_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::Address, Error> {
        Contract::get_contract_id(env, &contract_name.try_into()?)
    }

    /// Look up the owner of a deployed contract
    fn fetch_contract_owner(
        env: &Env,
        contract_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::Address, Error> {
        Contract::get_contract_owner(env, &contract_name.try_into()?)
    }
}

#[contracttrait]
pub trait Redeployable {
    /// Skips the publish step to deploy a contract directly, keeping the name
    fn dev_deploy(
        env: &Env,
        name: soroban_sdk::String,
        wasm: soroban_sdk::Bytes,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error> {
        let wasm_hash = env.deployer().upload_contract_wasm(wasm);
        Contract::upgrade_internal(env, &name.try_into()?, &wasm_hash, upgrade_fn)
    }

    /// Upgrades a contract by calling the upgrade function.
    /// Default is 'upgrade' and expects that first arg is the corresponding wasm hash
    fn upgrade_contract(
        env: &Env,
        name: soroban_sdk::String,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error> {
        let wasm_hash = Contract::get_hash_and_bump(env, &wasm_name.try_into()?, version)?;
        Contract::upgrade_internal(env, &name.try_into()?, &wasm_hash, upgrade_fn)
    }
}
