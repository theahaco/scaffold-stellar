use crate::name::NormalizedName;
use crate::storage::Storage;

use soroban_sdk::{self, contracttrait, contracttype, Address, BytesN, Env, Map, String};

use crate::{error::Error, Contract};

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
        Storage::new(env).hash.set(hash, &());
    }

    pub fn has(env: &Env, hash: &BytesN<32>) -> bool {
        Storage::new(env).hash.has(hash)
    }

    pub fn bump(env: &Env, hash: &BytesN<32>) {
        Storage::new(env).hash.extend_ttl_max(hash);
    }
}

impl Contract {
    fn registry(env: &Env, name: &NormalizedName) -> Result<PublishedWasm, Error> {
        Storage::new(env)
            .wasm
            .get(name)
            .ok_or(Error::NoSuchWasmPublished)
    }
    pub fn most_recent_version(env: &Env, name: &NormalizedName) -> Result<String, Error> {
        Ok(Self::registry(env, name)?.current_version)
    }

    pub(crate) fn get_version(
        env: &Env,
        name: &NormalizedName,
        version: Option<String>,
    ) -> Result<String, Error> {
        version
            .or_else(|| Self::most_recent_version(env, name).ok())
            .ok_or(Error::NoSuchWasmPublished)
    }

    pub(crate) fn get_hash(
        env: &Env,
        name: &NormalizedName,
        version: Option<String>,
    ) -> Result<BytesN<32>, Error> {
        Self::registry(env, name)?.get_hash(version)
    }

    pub(crate) fn get_hash_and_bump(
        env: &Env,
        name: &NormalizedName,
        version: Option<String>,
    ) -> Result<BytesN<32>, Error> {
        let registry = Self::registry(env, name)?;
        Storage::new(env).wasm.extend_ttl_max(name);
        let hash = registry.get_hash(version)?;
        HashMap::bump(env, &hash);
        Ok(hash)
    }

    pub(crate) fn set(
        env: &Env,
        name: &NormalizedName,
        version: &String,
        hash: &BytesN<32>,
        author: Address,
    ) {
        let mut wasm_map = Storage::new(env).wasm;
        let mut registry = wasm_map.get(name).unwrap_or_else(|| PublishedWasm {
            versions: Map::new(env),
            author,
            current_version: version.clone(),
        });
        registry.versions.set(version.clone(), hash.clone());
        registry.current_version = version.clone();
        wasm_map.set(name, &registry);
    }

    pub(crate) fn author(env: &Env, name: &NormalizedName) -> Option<Address> {
        Self::registry(env, name).ok().map(|wasm| wasm.author)
    }

    pub(crate) fn validate_version(
        env: &Env,
        version: &String,
        wasm_name: &NormalizedName,
    ) -> Result<(), Error> {
        let version = crate::version::parse(version)?;
        if let Ok(current_version) = Self::most_recent_version(env, wasm_name) {
            if version <= crate::version::parse(&current_version)? {
                return Err(Error::VersionMustBeGreaterThanCurrent);
            }
        }
        Ok(())
    }

    pub(crate) fn authorize(
        env: &Env,
        author: &Address,
        wasm_name: &NormalizedName,
    ) -> Result<(), Error> {
        // check if already published
        if let Some(current) = &Self::author(env, wasm_name) {
            if author != current {
                return Err(Error::WasmNameAlreadyTaken);
            }
            author.require_auth();
        } else if let Some(manager) = Storage::manager(env) {
            // Manager must approve initial Publish
            manager.require_auth();
        } else {
            author.require_auth();
        }
        Ok(())
    }
}

#[contracttrait]
pub trait Publishable {
    /// Fetch the hash of a Wasm binary from the registry
    fn fetch_hash(
        env: &Env,
        wasm_name: soroban_sdk::String,
        version: Option<soroban_sdk::String>,
    ) -> Result<soroban_sdk::BytesN<32>, Error> {
        Contract::get_hash(env, &wasm_name.try_into()?, version)
    }

    /// Most recent version of the published Wasm binary
    fn current_version(
        env: &Env,
        wasm_name: soroban_sdk::String,
    ) -> Result<soroban_sdk::String, Error> {
        Contract::most_recent_version(env, &wasm_name.try_into()?)
    }

    /// Publish a binary. Contract uploads bytes ensuring hash is correct.
    /// If contract had been previously published only previous author can publish again
    fn publish(
        env: &Env,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm: soroban_sdk::Bytes,
        version: soroban_sdk::String,
    ) -> Result<(), Error> {
        let wasm_hash = env.deployer().upload_contract_wasm(wasm);
        Contract::publish_hash(env, wasm_name, author, wasm_hash, version)
    }

    /// Publish a hash of a binary.
    /// If contract had been previously published only previous author can publish again
    fn publish_hash(
        env: &Env,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        version: soroban_sdk::String,
    ) -> Result<(), Error> {
        if HashMap::has(env, &wasm_hash) {
            return Err(Error::HashAlreadyPublished);
        }
        HashMap::add(env, &wasm_hash);
        let wasm_name = wasm_name.try_into()?;
        Contract::authorize(env, &author, &wasm_name)?;
        Contract::validate_version(env, &version, &wasm_name)?;
        Contract::set(env, &wasm_name, &version, &wasm_hash, author.clone());
        crate::events::Publish {
            wasm_name: wasm_name.to_string(),
            wasm_hash,
            version,
            author,
        }
        .publish(env);
        Ok(())
    }
}
