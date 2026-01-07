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
    use rand::{rngs::SmallRng, RngCore, SeedableRng};
    use soroban_sdk::{xdr::ToXdr, Env, IntoVal, String};

    #[test]
    fn hash_key_prefix_is_unique() {
        let env = &Env::default();
        env.cost_estimate().budget().reset_unlimited();
        let mut key_bytes: [u8; 32] = [0; 32];
        SmallRng::from_os_rng().fill_bytes(&mut key_bytes);
        let mut hash = env.crypto().sha256(&key_bytes.into_val(env));
        let hash_prefix: [u8; 4] = [0, 0, 0, 13];
        let vec_prefix: [u8; 4] = [0, 0, 0, 16];
        let mut key_prefix = [0u8; 4];
        // Verify over multiple iterations that the first 4 bytes of the XDR
        // serialization of the different key types remain distinct.
        for _ in 0..10_000 {
            let val = HashKey::to_key(env, &hash.to_bytes());
            let bytes = val.to_xdr(env);
            bytes.slice(..4).copy_into_slice(&mut key_prefix);
            assert_eq!(
                hash_prefix, key_prefix,
                "hash key will always have the same prefix as BytesN<32>"
            );
            let s = String::from_str(env, &std::format!("{}_hello", bytes.get(1).unwrap()));
            let wasm_key = WasmKey::to_key(env, &s).to_xdr(env);
            wasm_key.slice(..4).copy_into_slice(&mut key_prefix);
            assert_eq!(vec_prefix, key_prefix);
            let contract_key = ContractKey::to_key(env, &s).to_xdr(env);
            contract_key.slice(..4).copy_into_slice(&mut key_prefix);
            assert_eq!(vec_prefix, key_prefix);
            hash = env.crypto().sha256(&hash.to_bytes().into_val(env));
        }
    }
}
