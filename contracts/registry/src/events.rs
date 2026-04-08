use soroban_sdk::{contractevent, Address, BytesN, String};

#[contractevent(topics = ["register"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Register {
    pub contract_name: String,
    pub contract_id: Address,
    pub sac: bool,
    pub wasm_hash: Option<BytesN<32>>,
}

#[contractevent(topics = ["deploy"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deploy {
    pub wasm_name: String,
    pub version: String,
    pub deployer: Address,
    pub contract_id: Address,
}

#[contractevent(topics = ["publish"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Publish {
    pub wasm_name: String,
    pub wasm_hash: BytesN<32>,
    pub version: String,
    pub author: Address,
}

#[contractevent(topics = ["update_owner"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateOwner {
    pub contract_name: String,
    pub new_owner: Address,
}

#[contractevent(topics = ["update_address"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateAddress {
    pub contract_name: String,
    pub new_address: Address,
}

#[contractevent(topics = ["rename"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rename {
    pub old_name: String,
    pub new_name: String,
}

#[contractevent(topics = ["security_flag"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityFlagContract {
    pub is_compromised: bool,
}
