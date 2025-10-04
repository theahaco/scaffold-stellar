use loam_sdk::{
    loamstorage,
    soroban_sdk::{
        self, contracttype, env, to_string, Address, BytesN, Env, Map, PersistentMap, String,
    },
};
use loam_subcontract_core::Core as _;

use crate::{
    error::Error,
    name::canonicalize,
    util::{MAX_BUMP, REGISTRY},
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

/// Contains
#[loamstorage]
pub struct W {
    pub r: PersistentMap<String, PublishedWasm>,
}

impl W {
    fn registry(&self, name: &String) -> Result<PublishedWasm, Error> {
        self.r
            .get(name.clone())
            .ok_or(Error::NoSuchContractPublished)
    }
    pub fn most_recent_version(&self, name: &String) -> Result<String, Error> {
        self.r
            .get(name.clone())
            .map(|wasm| wasm.current_version)
            .ok_or(Error::NoSuchContractPublished)
    }

    pub fn get_version(&self, name: &String, version: Option<String>) -> Result<String, Error> {
        version
            .or_else(|| self.most_recent_version(name).ok())
            .ok_or(Error::NoSuchContractPublished)
    }

    pub fn get_hash(&self, name: &String, version: Option<String>) -> Result<BytesN<32>, Error> {
        self.registry(name)?.get_hash(version)
    }

    pub fn get_hash_and_bump(
        &mut self,
        name: &String,
        version: Option<String>,
    ) -> Result<BytesN<32>, Error> {
        let registry = self.registry(name)?;
        self.r.extend_ttl(name.clone(), MAX_BUMP, MAX_BUMP);
        let hash = registry.get_hash(version)?;
        HashMap::bump(env(), &hash);
        Ok(hash)
    }

    pub fn set(
        &mut self,
        name: &String,
        version: String,
        binary: BytesN<32>,
        author: Address,
    ) -> Result<(), Error> {
        let mut registry = self.r.get(name.clone()).unwrap_or_else(|| PublishedWasm {
            versions: Map::new(env()),
            author,
            current_version: version.clone(),
        });
        registry.versions.set(version.clone(), binary);
        registry.current_version = version;
        self.r.set(name.clone(), &registry);
        Ok(())
    }

    pub fn author(&self, name: &String) -> Option<Address> {
        self.registry(name).ok().map(|wasm| wasm.author)
    }

    fn validate_version(&self, version: &String, wasm_name: &String) -> Result<(), Error> {
        let version = crate::version::parse(version)?;
        if let Ok(current_version) = self.most_recent_version(wasm_name) {
            if version <= crate::version::parse(&current_version)? {
                return Err(Error::VersionMustBeGreaterThanCurrent);
            }
        }
        Ok(())
    }
}

impl IsPublishable for W {
    fn current_version(&self, wasm_name: String) -> Result<String, Error> {
        let wasm_name = canonicalize(&wasm_name)?;
        self.most_recent_version(&wasm_name)
    }

    fn publish(
        &mut self,
        wasm_name: String,
        author: Address,
        wasm: soroban_sdk::Bytes,
        version: String,
    ) -> Result<(), Error> {
        let wasm_hash = env().deployer().upload_contract_wasm(wasm);
        self.publish_hash(wasm_name, author, wasm_hash, version)
    }

    fn publish_hash(
        &mut self,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        version: String,
    ) -> Result<(), Error> {
        if HashMap::has(env(), &wasm_hash) {
            return Err(Error::HashAlreadyPublished);
        }
        HashMap::add(env(), &wasm_hash);
        author.require_auth();
        let wasm_name = canonicalize(&wasm_name)?;
        if let Some(current) = self.author(&wasm_name) {
            if author != current {
                return Err(Error::WasmNameAlreadyTaken);
            }
        }
        if wasm_name == to_string(REGISTRY) && crate::Contract::admin_get().unwrap() != author {
            return Err(Error::AdminOnly);
        }
        self.validate_version(&version, &wasm_name)?;
        self.set(&wasm_name, version.clone(), wasm_hash, author)?;
        Ok(())
    }

    fn fetch_hash(&self, wasm_name: String, version: Option<String>) -> Result<BytesN<32>, Error> {
        let wasm_name = canonicalize(&wasm_name)?;
        self.get_hash(&wasm_name, version)
    }
}
