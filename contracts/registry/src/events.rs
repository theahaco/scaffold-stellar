use soroban_sdk::{contractevent, Address, BytesN, String};

#[contractevent(topics = ["register"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Register {
    pub contract_name: String,
    pub contract_id: Address,
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
