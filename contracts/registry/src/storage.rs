use admin_sep::AdministratableExtension;
use soroban_sdk::{
    symbol_short,
    xdr::{ScErrorCode, ScErrorType},
    Address, BytesN, Env, IntoVal, TryFromVal, Val,
};
use soroban_sdk_tools::InstanceItem;

use crate::{
    name::NormalizedName,
    registry::{contract::DeployableClient, wasm::PublishedWasm},
    storage::maps::{ToStorageKey, MAX_BUMP},
    Contract, Error,
};

mod maps;

pub struct Storage {
    pub wasm: maps::PersistentMap<NormalizedName, PublishedWasm, WasmKey>,
    pub contract: maps::PersistentMap<NormalizedName, ContractEntry, ContractKey>,
    pub hash: maps::PersistentMap<BytesN<32>, (), HashKey>,
    pub root_registry: InstanceItem<Address>,
}

impl Storage {
    pub fn new(env: &Env) -> Self {
        Self {
            wasm: maps::PersistentMap::new(env),
            contract: maps::PersistentMap::new(env),
            hash: maps::PersistentMap::new(env),
            root_registry: InstanceItem::new_raw(env, symbol_short!("ROOT_REG").to_val()),
        }
    }
}

pub struct Manager;

impl ToStorageKey<()> for Manager {
    fn to_key(_: &Env, (): &()) -> Val {
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
        env.storage().instance().remove(&Manager::to_key(env, &()));
    }

    /// Resolves a subregistry name to its contract address via the trusted
    /// root. Subregistries pin the root's address at construction, so callers
    /// can't smuggle a forged address through `deploy_with_subregistry`. On
    /// the root itself we look up in local storage (Soroban disallows a
    /// contract calling itself, so xcc-to-self isn't an option).
    pub fn resolve_subregistry(
        env: &Env,
        subregistry: &soroban_sdk::String,
    ) -> Result<Address, Error> {
        let root = Storage::new(env).root_registry;
        if let Some(root_id) = root.get() {
            root.extend_ttl(MAX_BUMP, MAX_BUMP);
            let client = DeployableClient::new(env, &root_id);
            match client.try_fetch_contract_id(subregistry) {
                Ok(Ok(addr)) => Ok(addr),
                Err(Ok(e)) => Err(e),
                // Invoke aborts (root isn't a registry, panic) and
                // return-value conversion failures collapse to a single
                // opaque error.
                _ => Err(Error::SubRegistryCrossContractCallFailed),
            }
        } else {
            Contract::get_contract_id(env, &subregistry.try_into()?)
        }
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
    pub flagged: bool,
}

// `ContractEntry` is stored as either a 2-tuple (unflagged) or a 3-tuple with
// a `Void` sentinel (flagged). The *length* of the stored vec carries the
// flag, so unflagged entries — the common case on the hot proxy path — pay
// zero bytes of overhead for it.
impl IntoVal<Env, Val> for ContractEntry {
    fn into_val(&self, env: &Env) -> Val {
        if self.flagged {
            (self.owner.to_val(), self.contract.to_val(), ()).into_val(env)
        } else {
            (self.owner.to_val(), self.contract.to_val()).into_val(env)
        }
    }
}

impl TryFromVal<Env, Val> for ContractEntry {
    type Error = soroban_sdk::Error;

    fn try_from_val(env: &Env, v: &Val) -> Result<Self, soroban_sdk::Error> {
        // Decode to a `Vec<Val>` handle first and branch on length. A direct
        // tuple `TryFromVal` would go through `vec_unpack_to_linear_memory`,
        // which *traps* the VM on a length mismatch — so we couldn't recover
        // from a wrong guess. `vec_len` / `vec_get` return errors normally.
        let vec: soroban_sdk::Vec<Val> = TryFromVal::try_from_val(env, v)?;
        let flagged = match vec.len() {
            2 => false,
            3 => true,
            _ => {
                return Err(soroban_sdk::Error::from_type_and_code(
                    ScErrorType::Object,
                    ScErrorCode::UnexpectedSize,
                ))
            }
        };
        // Bounds already checked above, so these `get`s can be unchecked.
        let owner = TryFromVal::try_from_val(env, &vec.get_unchecked(0))?;
        let contract = TryFromVal::try_from_val(env, &vec.get_unchecked(1))?;
        Ok(ContractEntry {
            owner,
            contract,
            flagged,
        })
    }
}

/// ~1 week at 5s/ledger
pub const BATCH_TTL: u32 = 120_960;

impl Storage {
    pub fn get_batch(
        env: &Env,
    ) -> Option<soroban_sdk::Vec<(soroban_sdk::String, Address, Address)>> {
        let k = symbol_short!("BATCH").to_val();
        env.storage().temporary().get(&k)
    }

    pub fn set_batch(env: &Env, batch: &soroban_sdk::Vec<(soroban_sdk::String, Address, Address)>) {
        let k = symbol_short!("BATCH").to_val();
        env.storage().temporary().set(&k, batch);
        env.storage()
            .temporary()
            .extend_ttl(&k, BATCH_TTL, BATCH_TTL);
        Self::remove_batch_cursor(env);
    }

    pub fn remove_batch(env: &Env) {
        let k = symbol_short!("BATCH").to_val();
        env.storage().temporary().remove(&k);
    }

    pub fn batch_cursor(env: &Env) -> u32 {
        let k = symbol_short!("BATCHCUR").to_val();
        env.storage().temporary().get(&k).unwrap_or(0)
    }

    pub fn set_batch_cursor(env: &Env, cursor: u32) {
        let ck = symbol_short!("BATCHCUR").to_val();
        env.storage().temporary().set(&ck, &cursor);
        env.storage()
            .temporary()
            .extend_ttl(&ck, BATCH_TTL, BATCH_TTL);
        // Keep batch TTL in sync with cursor
        let bk = symbol_short!("BATCH").to_val();
        env.storage()
            .temporary()
            .extend_ttl(&bk, BATCH_TTL, BATCH_TTL);
    }

    pub fn remove_batch_cursor(env: &Env) {
        let k = symbol_short!("BATCHCUR").to_val();
        env.storage().temporary().remove(&k);
    }
}

impl From<ContractEntry> for (Address, Address, bool) {
    fn from(
        ContractEntry {
            owner,
            contract,
            flagged,
        }: ContractEntry,
    ) -> Self {
        (owner, contract, flagged)
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
