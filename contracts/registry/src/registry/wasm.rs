use crate::ContractArgs;
use crate::ContractClient;
use admin_sep::Administratable;
use soroban_sdk::{self, contractimpl, contracttype, Address, BytesN, Env, Map, String};

use crate::{
    error::Error,
    name::canonicalize,
    util::{MAX_BUMP, REGISTRY},
    Contract,
};

use super::IsPublishable;

#[contracttype(export = false)]
#[derive(Clone)]
pub struct PublishedWasm {
    pub versions: Map<String, BytesN<32>>,
    pub author: Address,
    pub current_version: String,
}

impl PublishedWasm {
    pub fn get_hash(&self, version: Option<String>) -> Result<BytesN<32>, Error> {
        self.versions
            .get(version.unwrap_or_else(|| self.current_version.clone()))
            .ok_or(Error::NoSuchVersion)
    }
}

pub struct HashMap;

impl HashMap {
    pub fn add(env: &Env, hash: &BytesN<32>) {
        env.storage().persistent().set(hash, &());
    }

    pub fn has(env: &Env, hash: &BytesN<32>) -> bool {
        env.storage().persistent().has(hash)
    }

    pub fn bump(env: &Env, hash: &BytesN<32>) {
        env.storage()
            .persistent()
            .extend_ttl(hash, MAX_BUMP, MAX_BUMP);
    }
}

#[contractimpl]
impl Contract {
    fn registry(env: &Env, name: &String) -> Result<PublishedWasm, Error> {
        env.storage()
            .persistent()
            .get(&name.clone().to_val())
            .ok_or(Error::NoSuchContractPublished)
    }
    pub fn most_recent_version(env: &Env, name: &String) -> Result<String, Error> {
        env.storage()
            .persistent()
            .get(&name.clone().to_val())
            .map(|wasm: PublishedWasm| wasm.current_version)
            .ok_or(Error::NoSuchContractPublished)
    }

    pub fn get_version(env: &Env, name: &String, version: Option<String>) -> Result<String, Error> {
        version
            .or_else(|| Self::most_recent_version(env, name).ok())
            .ok_or(Error::NoSuchContractPublished)
    }

    pub fn get_hash(
        env: &Env,
        name: &String,
        version: Option<String>,
    ) -> Result<BytesN<32>, Error> {
        Self::registry(env, name)?.get_hash(version)
    }

    pub fn get_hash_and_bump(
        env: &Env,
        name: &String,
        version: Option<String>,
    ) -> Result<BytesN<32>, Error> {
        let registry = Self::registry(env, name)?;
        env.storage()
            .persistent()
            .extend_ttl(&name.clone().to_val(), MAX_BUMP, MAX_BUMP);
        let hash = registry.get_hash(version)?;
        HashMap::bump(env, &hash);
        Ok(hash)
    }

    pub fn set(
        env: &Env,
        name: &String,
        version: String,
        binary: BytesN<32>,
        author: Address,
    ) -> Result<(), Error> {
        let mut registry = env
            .storage()
            .persistent()
            .get(&name.clone().to_val())
            .unwrap_or_else(|| PublishedWasm {
                versions: Map::new(env),
                author,
                current_version: version.clone(),
            });
        registry.versions.set(version.clone(), binary);
        registry.current_version = version;
        env.storage()
            .persistent()
            .set(&name.clone().to_val(), &registry);
        Ok(())
    }

    pub fn author(env: &Env, name: &String) -> Option<Address> {
        Self::registry(env, name).ok().map(|wasm| wasm.author)
    }

    fn validate_version(env: &Env, version: &String, wasm_name: &String) -> Result<(), Error> {
        let version = crate::version::parse(version)?;
        if let Ok(current_version) = Self::most_recent_version(env, wasm_name) {
            if version <= crate::version::parse(&current_version)? {
                return Err(Error::VersionMustBeGreaterThanCurrent);
            }
        }
        Ok(())
    }
}

#[contractimpl]
impl IsPublishable for Contract {
    fn current_version(env: &Env, wasm_name: String) -> Result<String, Error> {
        let wasm_name = canonicalize(&wasm_name)?;
        Self::most_recent_version(env, &wasm_name)
    }

    fn publish(
        env: &Env,
        wasm_name: String,
        author: Address,
        wasm: soroban_sdk::Bytes,
        version: String,
    ) -> Result<(), Error> {
        let wasm_hash = env.deployer().upload_contract_wasm(wasm);
        Self::publish_hash(env, wasm_name, author, wasm_hash, version)
    }

    fn publish_hash(
        env: &Env,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        version: String,
    ) -> Result<(), Error> {
        if HashMap::has(env(), &wasm_hash) {
            return Err(Error::HashAlreadyPublished);
        }
        HashMap::add(env, &wasm_hash);
        author.require_auth();
        let wasm_name = canonicalize(&wasm_name)?;
        if let Some(current) = Self::author(env, &wasm_name) {
            if author != current {
                return Err(Error::WasmNameAlreadyTaken);
            }
        }
        let str = soroban_sdk::String::from_str(env, REGISTRY);
        if wasm_name == str && Self::admin_from_storage(env).unwrap() != author {
            return Err(Error::AdminOnly);
        }
        Self::validate_version(env, &version, &wasm_name)?;
        Self::set(env, &wasm_name, version.clone(), wasm_hash, author)?;
        Ok(())
    }

    fn fetch_hash(
        env: &Env,
        wasm_name: String,
        version: Option<String>,
    ) -> Result<BytesN<32>, Error> {
        let wasm_name = canonicalize(&wasm_name)?;
        Self::get_hash(env, &wasm_name, version)
    }
}
