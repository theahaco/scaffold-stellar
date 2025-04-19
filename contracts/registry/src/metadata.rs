use loam_sdk::soroban_sdk::{self, contracttype, BytesN, String};

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Default)]
#[contracttype]
pub struct Metadata {
    pub repo: Option<String>,
}

impl Metadata {
    pub fn new(repo: String) -> Self {
        Self { repo: Some(repo) }
    }
}

/// Contains info about specific version of published binary
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct PublishedWasm {
    pub hash: BytesN<32>,
    pub metadata: Metadata,
}
