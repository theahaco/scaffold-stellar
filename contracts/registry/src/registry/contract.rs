#![allow(non_upper_case_globals)]
use crate::name;
use crate::name::NormalizedName;
use crate::name::UNVERIFIED;
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
use admin_sep::AdministratableExtension;

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

    pub(crate) fn require_owner_or_manager(env: &Env, owner: &Address) {
        if let Some(manager) = Storage::manager(env) {
            manager.require_auth();
        } else {
            owner.require_auth();
        }
    }

    pub(crate) fn register_contract_name(
        env: &Env,
        contract_name: &NormalizedName,
        contract_id: &Address,
        contract_admin: &Address,
    ) -> Result<(), Error> {
        let mut contract_map = Storage::new(env).contract;
        contract_map.set(
            contract_name,
            &ContractEntry {
                owner: contract_admin.clone(),
                contract: contract_id.clone(),
                flagged: false,
            },
        );
        let wasm_hash = match contract_id
            .executable()
            .ok_or(Error::ContractIdAddressDoesNotExist)?
        {
            Executable::Wasm(bytes_n) => Some(bytes_n),
            Executable::StellarAsset => None,
            Executable::Account => return Err(Error::AccountAddressNotValid),
        };
        crate::events::Register {
            contract_name: contract_name.to_string(),
            contract_id: contract_id.clone(),
            sac: wasm_hash.is_none(),
            wasm_hash,
        }
        .publish(env);
        Ok(())
    }

    pub(crate) fn fetch_hash_and_deploy(
        env: &Env,
        wasm_name: &NormalizedName,
        version: Option<String>,
        salt: impl IntoVal<Env, BytesN<32>>,
        init: Option<Vec<Val>>,
        deployer: Address,
    ) -> Result<Address, Error> {
        let hash = Self::get_hash_and_bump(env, wasm_name, version.clone())?;
        let contract_id = deploy_and_init(env, salt, hash, init, deployer.clone());
        let version = Self::get_version(env, wasm_name, version)?;
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
    /// Furthermore it uses the `NormalizedName::new_unchecked`, which is unsafe because it skips validating
    /// the name, which we know already to be valid.
    pub(crate) fn deploy_unverified_and_claim_registry(
        env: &Env,
        admin: &Address,
    ) -> Result<(), Error> {
        unsafe {
            if let Executable::Wasm(wasm_hash) = env
                .current_contract_address()
                .executable()
                .unwrap_unchecked()
            {
                let contract_name =
                    NormalizedName::new_unchecked(String::from_str(env, UNVERIFIED));
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
                Self::register_contract_name(env, &contract_name, &contract_address, admin)?;
                Self::register_contract_name(
                    env,
                    &name::registry(env),
                    &env.current_contract_address(),
                    admin,
                )?;
            }
            Ok(())
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
            &wasm_name.try_into()?,
            version.clone(),
            salt,
            init,
            deployer.clone(),
        )?;
        Contract::register_contract_name(env, &contract_name, &contract_id, &admin)?;
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
        Contract::fetch_hash_and_deploy(env, &wasm_name.try_into()?, version, salt, init, deployer)
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
        Contract::register_contract_name(env, &contract_name, &contract_address, &owner)?;
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
pub trait Batchable {
    /// Stage a batch of existing contracts for registration.
    /// Requires manager auth if manager is set, otherwise admin auth.
    /// Each entry is (`contract_name`, `contract_address`, `owner`).
    /// The entire batch is stored in a single write after validation.
    fn batch_register(
        env: &Env,
        contracts: soroban_sdk::Vec<(
            soroban_sdk::String,
            soroban_sdk::Address,
            soroban_sdk::Address,
        )>,
    ) -> Result<(), Error> {
        if let Some(manager) = Storage::manager(env) {
            manager.require_auth();
        } else {
            Contract::require_admin(env);
        }

        let contract_map = Storage::new(env).contract;
        let mut seen: soroban_sdk::Map<soroban_sdk::String, ()> = soroban_sdk::Map::new(env);

        for entry in contracts.iter() {
            let (name_str, _contract_address, _owner) = entry;
            let contract_name: NormalizedName = name_str.try_into()?;
            let name_key = contract_name.to_string();

            if contract_map.has(&contract_name) {
                return Err(Error::AlreadyDeployed);
            }

            if seen.contains_key(name_key.clone()) {
                return Err(Error::AlreadyDeployed);
            }
            seen.set(name_key, ());
        }

        Storage::set_batch(env, &contracts);
        Ok(())
    }

    /// Process up to `limit` pending batch entries, registering each contract.
    /// Callable by anyone. Returns the number of contracts processed.
    /// Call repeatedly to iterate through all entries.
    fn process_batch(env: &Env, limit: u32) -> Result<u32, Error> {
        let batch = Storage::get_batch(env).ok_or(Error::NoPendingBatch)?;
        let len = batch.len();
        let cursor = Storage::batch_cursor(env);

        if cursor >= len {
            return Err(Error::NoPendingBatch);
        }

        let end = (cursor + limit).min(len);
        let mut processed = 0u32;

        for i in cursor..end {
            let (name_str, contract_address, owner) =
                batch.get(i).ok_or(Error::BatchEntryExpired)?;
            let contract_name: NormalizedName = name_str.try_into()?;
            Contract::register_contract_name(env, &contract_name, &contract_address, &owner)?;
            processed += 1;
        }

        if end >= len {
            Storage::remove_batch(env);
            Storage::remove_batch_cursor(env);
        } else {
            Storage::set_batch_cursor(env, end);
        }

        Ok(processed)
    }
}

#[contracttrait]
pub trait Manageable {
    /// Update the owner of a registered contract.
    /// Requires current owner auth, or manager auth if manager is set.
    fn update_contract_owner(
        env: &Env,
        contract_name: soroban_sdk::String,
        new_owner: soroban_sdk::Address,
    ) -> Result<(), Error> {
        let contract_name: NormalizedName = contract_name.try_into()?;
        let mut storage = Storage::new(env);
        let entry = storage
            .contract
            .get(&contract_name)
            .ok_or(Error::NoSuchContractDeployed)?;

        Contract::require_owner_or_manager(env, &entry.owner);

        storage.contract.extend_ttl_max(&contract_name);
        storage.contract.set(
            &contract_name,
            &ContractEntry {
                owner: new_owner.clone(),
                contract: entry.contract,
                flagged: entry.flagged,
            },
        );
        crate::events::UpdateOwner {
            contract_name: contract_name.to_string(),
            new_owner,
        }
        .publish(env);
        Ok(())
    }

    /// Update the contract address of a registered contract.
    /// Requires current owner auth, or manager auth if manager is set.
    fn update_contract_address(
        env: &Env,
        contract_name: soroban_sdk::String,
        new_address: soroban_sdk::Address,
    ) -> Result<(), Error> {
        let contract_name: NormalizedName = contract_name.try_into()?;
        let mut storage = Storage::new(env);
        let entry = storage
            .contract
            .get(&contract_name)
            .ok_or(Error::NoSuchContractDeployed)?;

        Contract::require_owner_or_manager(env, &entry.owner);
        storage.contract.extend_ttl_max(&contract_name);
        storage.contract.set(
            &contract_name,
            &ContractEntry {
                owner: entry.owner,
                contract: new_address.clone(),
                flagged: entry.flagged,
            },
        );
        crate::events::UpdateAddress {
            contract_name: contract_name.to_string(),
            new_address,
        }
        .publish(env);
        Ok(())
    }

    /// Rename a registered contract.
    /// Requires current owner auth, or manager auth if manager is set.
    fn rename_contract(
        env: &Env,
        old_name: soroban_sdk::String,
        new_name: soroban_sdk::String,
    ) -> Result<(), Error> {
        let old_name: NormalizedName = old_name.try_into()?;
        let new_name: NormalizedName = new_name.try_into()?;

        let mut storage = Storage::new(env);
        let entry = storage
            .contract
            .get(&old_name)
            .ok_or(Error::NoSuchContractDeployed)?;

        Contract::require_owner_or_manager(env, &entry.owner);

        if storage.contract.has(&new_name) {
            return Err(Error::AlreadyDeployed);
        }

        storage.contract.remove(&old_name);
        storage.contract.set(&new_name, &entry);
        storage.contract.extend_ttl_max(&new_name);

        crate::events::Rename {
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
        }
        .publish(env);
        Ok(())
    }

    /// Flag contract, marking contract as compromised or
    /// un-marking it as being compromised
    fn flag_contract(
        env: &Env,
        contract_name: soroban_sdk::String,
        flagged: bool,
    ) -> Result<(), Error> {
        let contract_name: NormalizedName = contract_name.try_into()?;

        let mut storage = Storage::new(env);
        let entry = Contract::get_contract_entry(env, &contract_name)?;

        Contract::require_owner_or_manager(env, &entry.owner);

        storage.contract.extend_ttl_max(&contract_name);
        storage.contract.set(
            &contract_name,
            &ContractEntry {
                owner: entry.owner,
                contract: entry.contract,
                flagged,
            },
        );

        crate::events::SecurityFlagContract { flagged }.publish(env);
        Ok(())
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

#[contracttrait]
pub trait Proxyable {
    /// Invokes contract with the given contract name, using given function name and arguments
    fn proxy_invoke_contract(
        env: &Env,
        contract_name: soroban_sdk::String,
        contract_fn: soroban_sdk::Symbol,
        args: Vec<Val>,
    ) -> Result<soroban_sdk::Val, Error> {
        let contract_name: NormalizedName = contract_name.try_into()?;
        let entry = Contract::get_contract_entry(env, &contract_name)?;
        if entry.flagged {
            return Err(Error::ProxyContractCompromised);
        }
        if let Ok(Ok(ok_result)) =
            env.try_invoke_contract::<Val, InvokeError>(&entry.contract, &contract_fn, args)
        {
            Ok(ok_result)
        } else {
            Err(Error::ProxyInvocationFailed)
        }
    }
}
