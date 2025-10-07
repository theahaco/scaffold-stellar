use soroban_sdk::{Address, BytesN, String, contractevent};

// Define the event using the `contractevent` attribute macro.
#[contractevent(topics = ["deploy"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deploy {
    pub wasm_name: String,
    pub contract_name: String,
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
