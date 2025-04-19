use loam_sdk::{
    loamstorage,
    soroban_sdk::{self, env, Address, PersistentMap, String},
};

use crate::{
    error::Error,
    metadata::{Metadata, PublishedWasm},
    version::{self, Version, INITAL_VERSION},
};

use super::IsPublishable;

/// Contains
#[loamstorage]
pub struct Wasm {
    pub versions: PersistentMap<(String, Version), PublishedWasm>,
    pub author: PersistentMap<String, Address>,
    pub most_recent_version: PersistentMap<String, Version>,
}

impl Wasm {
    pub fn new(name: &String, author: Address) -> Self {
        let mut s = Self::default();
        s.author.set(name.clone(), &author);
        s
    }
}

impl Wasm {
    pub fn most_recent_version(&self, name: &String) -> Result<Version, Error> {
        self.most_recent_version
            .get(name.clone())
            .ok_or(Error::NoSuchVersion)
    }

    pub fn set_most_recent_version(&mut self, name: &String, version: Version) {
        self.most_recent_version.set(name.clone(), &version);
    }

    pub fn get(&self, name: &String, version: Option<Version>) -> Result<PublishedWasm, Error> {
        let version = if let Some(version) = version {
            version
        } else {
            self.most_recent_version(name)?
        };
        self.versions
            .get((name.clone(), version))
            .ok_or(Error::NoSuchVersion)
    }

    pub fn set(
        &mut self,
        name: String,
        version: Option<Version>,
        binary: PublishedWasm,
    ) -> Result<(), Error> {
        let version = if let Some(version) = version {
            version
        } else {
            self.most_recent_version(&name)?
        };
        self.versions.set((name, version), &binary);
        Ok(())
    }

    pub fn author(&self, name: &String) -> Option<Address> {
        self.author.get(name.clone())
    }
}

impl IsPublishable for Wasm {
    fn fetch(
        &self,
        contract_name: String,
        version: Option<Version>,
    ) -> Result<PublishedWasm, Error> {
        self.get(&contract_name, version)
    }

    fn current_version(&self, contract_name: String) -> Result<Version, Error> {
        self.most_recent_version(&contract_name)
    }

    fn publish(
        &mut self,
        wasm_name: String,
        author: Address,
        wasm: soroban_sdk::Bytes,
        repo: Option<String>,
        kind: Option<version::Update>,
    ) -> Result<(), Error> {
        let wasm_hash = env().deployer().upload_contract_wasm(wasm);
        self.publish_hash(wasm_name, author, wasm_hash, repo, kind)
    }

    fn publish_hash(
        &mut self,
        wasm_name: soroban_sdk::String,
        author: soroban_sdk::Address,
        wasm_hash: soroban_sdk::BytesN<32>,
        repo: Option<soroban_sdk::String>,
        kind: Option<version::Update>,
    ) -> Result<(), Error> {
        if let Some(cunnet_author) = self.author(&wasm_name) {
            if author != cunnet_author {
                return Err(Error::AlreadyPublished);
            }
        }

        author.require_auth();

        let last_version: Version = self.most_recent_version(&wasm_name).unwrap_or_default();
        last_version.log();
        let new_version = last_version.clone().update(&kind.unwrap_or_default());
        new_version.log();

        let metadata = if let Some(repo) = repo {
            Metadata::new(repo)
        } else if new_version == INITAL_VERSION {
            Metadata::default()
        } else {
            self.get(&wasm_name, Some(last_version))?.metadata
        };
        let published_binary = PublishedWasm {
            hash: wasm_hash,
            metadata,
        };
        self.set_most_recent_version(&wasm_name, new_version.clone());
        self.set(wasm_name, Some(new_version), published_binary)
    }
}
