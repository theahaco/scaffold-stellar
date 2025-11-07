use soroban_sdk::{symbol_short, Address, BytesN, Env, IntoVal, String, Val};

use crate::{registry::wasm::PublishedWasm, storage::maps::ToStorageKey};

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

impl ToStorageKey<String> for ContractKey {
    fn to_key(env: &Env, k: &String) -> Val {
        (symbol_short!("CR"), k.clone()).into_val(env)
    }
}

pub struct WasmKey;

impl ToStorageKey<String> for WasmKey {
    fn to_key(env: &Env, k: &String) -> Val {
        (symbol_short!("WA"), k.clone()).into_val(env)
    }
}

pub struct HashKey;

impl ToStorageKey<BytesN<32>> for HashKey {
    fn to_key(_: &Env, k: &BytesN<32>) -> Val {
        k.to_val()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::{maps::ToStorageKey, ContractKey, HashKey, WasmKey};
    use soroban_sdk::{xdr::ToXdr, Env, IntoVal, String};

    #[test]
    fn test_hash_key() {
        let env = &Env::default();
        let key_bytes: [u8; 32] = [1; 32];
        let mut hash = env.crypto().sha256(&key_bytes.into_val(env));
        let hash_prefix: [u8; 4] = [0, 0, 0, 13];
        let vec_prefix: [u8; 4] = [0, 0, 0, 16];
        let mut v = [0u8; 4];
        for _ in 0..10_000 {
            let val = HashKey::to_key(env, &hash.to_bytes());
            let bytes = val.to_xdr(env);
            bytes.slice(..4).copy_into_slice(&mut v);
            assert_eq!(hash_prefix, v);
            let s = String::from_str(env, &std::format!("hello{}", bytes.get(1).unwrap()));
            let expected_tuple = WasmKey::to_key(env, &s).to_xdr(env);
            expected_tuple.slice(..4).copy_into_slice(&mut v);
            assert_eq!(vec_prefix, v);
            let expected_tuple = ContractKey::to_key(env, &s).to_xdr(env);
            expected_tuple.slice(..4).copy_into_slice(&mut v);
            assert_eq!(vec_prefix, v);
            hash = env.crypto().sha256(&hash.to_bytes().into_val(env));
        }
    }
}
