use admin_sep::AdministratableExtension;
use soroban_sdk::{symbol_short, Address, BytesN, Env, IntoVal, Val};

use crate::{
    name::NormalizedName, registry::wasm::PublishedWasm, storage::maps::ToStorageKey, Contract,
};

mod maps;

pub struct Storage {
    pub wasm: maps::PersistentMap<NormalizedName, PublishedWasm, WasmKey>,
    pub contract: maps::PersistentMap<NormalizedName, (Address, Address), ContractKey>,
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

pub struct Manager;

impl ToStorageKey<()> for Manager {
    fn to_key(_: &Env, _: &()) -> Val {
        symbol_short!("MANAGER").to_val()
    }
}

impl Storage {
    pub fn manager(env: &Env) -> Option<Address> {
        env.storage().instance().get(&Manager::to_key(env, &()))
    }
    pub fn set_manager_no_auth(env: &Env, new_manager: &Address) {
        env.storage()
            .instance()
            .set(&Manager::to_key(env, &()), new_manager);
    }
    pub fn set_manager(env: &Env, new_manager: &Address) {
        Contract::require_admin(env);
        Self::set_manager_no_auth(env, new_manager);
    }

    pub fn remove_manager(env: &Env) {
        Contract::require_admin(env);
        env.storage().instance().remove(&Manager::to_key(env, &()))
    }
}

pub struct ContractKey;

impl ToStorageKey<NormalizedName> for ContractKey {
    fn to_key(env: &Env, k: &NormalizedName) -> Val {
        (symbol_short!("CR"), k.to_string()).into_val(env)
    }
}

pub struct WasmKey;

impl ToStorageKey<NormalizedName> for WasmKey {
    fn to_key(env: &Env, k: &NormalizedName) -> Val {
        (symbol_short!("WA"), k.to_string()).into_val(env)
    }
}

pub struct HashKey;

impl ToStorageKey<BytesN<32>> for HashKey {
    fn to_key(_: &Env, k: &BytesN<32>) -> Val {
        k.to_val()
    }
}

#[derive(Clone)]
pub struct ContractEntry {
    pub owner: Address,
    pub contract: Address,
}

impl From<(Address, Address)> for ContractEntry {
    fn from((owner, contract): (Address, Address)) -> Self {
        ContractEntry { owner, contract }
    }
}

impl From<ContractEntry> for (Address, Address) {
    fn from(ContractEntry { owner, contract }: ContractEntry) -> Self {
        (owner, contract)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use crate::name::NormalizedName;

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
            let s = &String::from_str(env, &std::format!("{}_hello", bytes.get(1).unwrap()));
            let wasm_key =
                WasmKey::to_key(env, unsafe { &NormalizedName::new_unchecked(s.clone()) })
                    .to_xdr(env);
            wasm_key.slice(..4).copy_into_slice(&mut key_prefix);
            assert_eq!(vec_prefix, key_prefix);
            let contract_key =
                ContractKey::to_key(env, unsafe { &NormalizedName::new_unchecked(s.clone()) })
                    .to_xdr(env);
            contract_key.slice(..4).copy_into_slice(&mut key_prefix);
            assert_eq!(vec_prefix, key_prefix);
            hash = env.crypto().sha256(&hash.to_bytes().into_val(env));
        }
    }
}
