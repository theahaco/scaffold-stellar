use loam_sdk::{
    loamstorage,
    soroban_sdk::{self, env, Address, BytesN, Map, PersistentMap, String},
};

use crate::{error::Error, version::Version};

use super::IsPublishable;

/// Contains
#[loamstorage]
pub struct Wasm {
    pub registry: PersistentMap<String, Map<Version, BytesN<32>>>,
    pub author: PersistentMap<String, Address>,
}

impl Wasm {
    pub fn new(name: &String, author: Address) -> Self {
        let mut s = Self::default();
        s.author.set(name.clone(), &author);
        s
    }
}

impl Wasm {
    fn registry(&self, name: &String) -> Result<Map<Version, BytesN<32>>, Error> {
        self.registry
            .get(name.clone())
            .ok_or(Error::NoSuchContractPublished)
    }
    pub fn most_recent_version(&self, name: &String) -> Result<Version, Error> {
        self.registry(name)?
            .keys()
            .first()
            .ok_or(Error::NoSuchVersion)
    }

    pub fn get(&self, name: &String, version: Option<Version>) -> Result<BytesN<32>, Error> {
        let registry = self.registry(name)?;
        if let Some(version) = version {
            registry.get(version)
        } else {
            registry.values().last()
        }
        .ok_or(Error::NoSuchVersion)
    }

    pub fn set(
        &mut self,
        name: &String,
        version: Version,
        binary: BytesN<32>,
    ) -> Result<(), Error> {
        let mut registry = self
            .registry
            .get(name.clone())
            .unwrap_or_else(|| Map::new(env()));
        registry.set(version, binary);
        self.registry.set(name.clone(), &registry);
        Ok(())
    }

    pub fn author(&self, name: &String) -> Option<Address> {
        self.author.get(name.clone())
    }
}

impl IsPublishable for Wasm {
    fn current_version(&self, contract_name: String) -> Result<Version, Error> {
        self.most_recent_version(&contract_name)
    }

    fn publish(
        &mut self,
        wasm_name: String,
        author: Address,
        wasm: soroban_sdk::Bytes,
        version: Version,
    ) -> Result<(), Error> {
        let wasm_hash = env().deployer().upload_contract_wasm(wasm);
        self.publish_hash(wasm_name, author, wasm_hash, version)
    }

    fn publish_hash(
        &mut self,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        version: Version,
    ) -> Result<(), Error> {
        if let Some(current) = self.author(&wasm_name) {
            if author != current {
                return Err(Error::AlreadyPublished);
            }
        }
        author.require_auth();
        version.log();
        self.author.set(wasm_name.clone(), &author);
        self.set(&wasm_name, version, wasm_hash)
    }

    fn fetch_hash(
        &self,
        contract_name: String,
        version: Option<Version>,
    ) -> Result<BytesN<32>, Error> {
        self.get(&contract_name, version)
    }
}
