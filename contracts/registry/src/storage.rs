use soroban_sdk::{symbol_short, Address, BytesN, Env, IntoVal, String, Val};

use crate::{registry::wasm::PublishedWasm, storage::maps::LoamKey};

mod maps;

pub struct Storage {
    pub wasm: maps::PersistentMap<String, PublishedWasm, WasmKey>,
    pub contract: maps::PersistentMap<String, Address, ContractKey>,
    pub hash: maps::PersistentMap<BytesN<32>, (), HashKey>,
}

impl Storage {
    pub fn new(env: &Env) -> Self {
        Self {
            wasm: maps::PersistentMap::new(env),
            contract: maps::PersistentMap::new(env),
            hash: maps::PersistentMap::new(env),
        }
    }
}

pub struct ContractKey;

impl LoamKey<String> for ContractKey {
    fn to_key(env: &Env, k: &String) -> Val {
        (symbol_short!("CR"), k.clone()).into_val(env)
    }
}

pub struct WasmKey;

impl LoamKey<String> for WasmKey {
    fn to_key(env: &Env, k: &String) -> Val {
        (symbol_short!("WA"), k.clone()).into_val(env)
    }
}

pub struct HashKey;

impl LoamKey<BytesN<32>> for HashKey {
    fn to_key(_: &Env, k: &BytesN<32>) -> Val {
        k.clone().to_val()
    }
}
