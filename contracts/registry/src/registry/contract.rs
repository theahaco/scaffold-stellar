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

use super::{wasm::Wasm, IsClaimable, IsDeployable, IsDevDeployable};

loam_sdk::import_contract!(example_core);

// Is the same as

// mod example_core {
//     use loam_sdk::soroban_sdk;
//     loam_sdk::soroban_sdk::contractimport!(file = "../../target/loam/example_core.wasm",);
// }

#[contracttype]
pub struct DeployEventData {
    published_name: String,
    deployed_name: String,
    version: Version,
    deployer: Address,
    contract_id: Address,
}

#[loamstorage]
pub struct Contract {
    pub registry: PersistentMap<String, ContractType>,
}

#[allow(clippy::module_name_repetitions)]
#[contracttype(export = false)]
#[derive(Clone)]
pub enum ContractType {
    Id(Address),
    IdAndOwner(Address, Address),
}

impl ContractType {
    pub fn contract_id(&self) -> &Address {
        match self {
            Self::Id(id) | Self::IdAndOwner(id, _) => id,
        }
    }
    pub fn owner(&self) -> Option<&Address> {
        match self {
            Self::IdAndOwner(_, owner) => Some(owner),
            Self::Id(_) => None,
        }
    }
}

impl IsDeployable for Contract {
    fn deploy(
        &mut self,
        contract_name: String,
        version: Option<Version>,
        deployed_name: String,
        owner: Address,
        salt: Option<BytesN<32>>,
        init: Option<(Symbol, soroban_sdk::Vec<soroban_sdk::Val>)>,
    ) -> Result<Address, Error> {
        let env = env();
        if self.registry.has(deployed_name.clone()) {
            return Err(Error::NoSuchContractDeployed);
        }
        // signed by owner
        owner.require_auth();
        let hash = Contract_::fetch_hash(contract_name.clone(), version.clone())?;
        let salt: BytesN<32> = salt.unwrap_or_else(|| hash_string(&deployed_name).into_val(env));
        let address = deploy_and_init(&owner, salt, hash)?;
        if let Some((init_fn, args)) = init {
            let _ = env.invoke_contract::<Val>(&address, &init_fn, args);
        }
        self.registry
            .set(deployed_name.clone(), &ContractType::Id(address.clone()));

        // Publish a deploy event
        let version =
            version.map_or_else(|| Wasm::default().most_recent_version(&contract_name), Ok)?;
        let deploy_datas = DeployEventData {
            published_name: contract_name,
            deployed_name,
            version,
            deployer: owner,
            contract_id: address.clone(),
        };
        env.events()
            .publish((symbol_short!("deploy"),), deploy_datas);

        Ok(address)
    }

    fn fetch_contract_id(&self, deployed_name: String) -> Result<Address, Error> {
        self.registry
            .get(deployed_name)
            .ok_or(Error::NoSuchContractDeployed)
            .map(|contract| contract.contract_id().clone())
    }
}

impl IsClaimable for Contract {
    fn claim_already_deployed_contract(
        &mut self,
        deployed_name: soroban_sdk::String,
        id: soroban_sdk::Address,
        owner: soroban_sdk::Address,
    ) -> Result<(), Error> {
        owner.require_auth();
        if self.registry.has(deployed_name.clone()) {
            return Err(Error::AlreadyClaimed);
        }
        self.registry
            .set(deployed_name, &ContractType::IdAndOwner(id, owner));
        Ok(())
    }

    fn get_claimed_owner(
        &self,
        deployed_name: soroban_sdk::String,
    ) -> Result<Option<Address>, Error> {
        self.registry
            .get(deployed_name)
            .ok_or(Error::NoSuchContractDeployed)
            .map(|contract| contract.owner().cloned())
    }

    fn redeploy_claimed_contract(
        &self,
        binary_name: Option<soroban_sdk::String>,
        version: Option<Version>,
        deployed_name: soroban_sdk::String,
        redeploy_fn: Option<(soroban_sdk::Symbol, soroban_sdk::Vec<soroban_sdk::Val>)>,
    ) -> Result<(), Error> {
        self.get_claimed_owner(deployed_name.clone())?
            .ok_or(Error::NoOwnerSet)?
            .require_auth();
        let contract_id = self.fetch_contract_id(deployed_name)?;
        if let Some(binary_name) = binary_name {
            let hash = Contract_::fetch_hash(binary_name, version)?;
            env().deployer().update_current_contract_wasm(hash);
        } else if let Some((fn_name, args)) = redeploy_fn {
            let _ = env().invoke_contract::<Val>(&contract_id, &fn_name, args);
        } else {
            return Err(Error::RedeployDeployedFailed);
        }
        Ok(())
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
        if let Some(contract_state) = self.registry.get(name.clone()) {
            let address = contract_state.contract_id();
            let contract = example_core::Client::new(env(), address);
            contract.redeploy(&wasm_hash);
            return Ok(address.clone());
        }
        let salt = hash_string(&name);
        let id = deploy_and_init(&owner, salt, wasm_hash)?;
        self.registry.set(name, &ContractType::Id(id.clone()));
        Ok(id)
    }
}
