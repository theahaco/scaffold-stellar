use loam_sdk::{
    loamstorage,
    soroban_sdk::{
        self, assert_with_error, env, to_string, Address, BytesN, Map, PersistentMap, String,
    },
};
use loam_subcontract_core::Core as _;

use crate::{error::Error, util::REGISTRY, version::Version};

use super::IsPublishable;

/// Contains
#[loamstorage]
pub struct W {
    pub r: PersistentMap<String, Map<Version, BytesN<32>>>,
    pub a: PersistentMap<String, Address>,
}

impl W {
    pub fn new(name: &String, author: Address) -> Self {
        let mut s = Self::default();
        s.a.set(name.clone(), &author);
        s
    }
}

impl W {
    fn registry(&self, name: &String) -> Result<Map<Version, BytesN<32>>, Error> {
        self.r
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
        let mut registry = self.r.get(name.clone()).unwrap_or_else(|| Map::new(env()));
        registry.set(version, binary);
        self.r.set(name.clone(), &registry);
        Ok(())
    }

    pub fn author(&self, name: &String) -> Option<Address> {
        self.a.get(name.clone())
    }
}

impl IsPublishable for W {
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
        if wasm_name == to_string(REGISTRY) {
            assert_with_error!(
                env(),
                crate::Contract::admin_get().unwrap() == author,
                Error::AdminOnly
            );
        }
        author.require_auth();
        version.log();
        if let Ok(current_version) = self.most_recent_version(&wasm_name) {
            assert_with_error!(
                env(),
                version > current_version,
                Error::VersionMustBeGreaterThanCurrent
            );
        };
        self.a.set(wasm_name.clone(), &author);
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
