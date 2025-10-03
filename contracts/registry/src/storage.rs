use soroban_sdk::{contracttype, String};

#[contracttype]
pub enum DataKey {
    Wasm(String),
    Contract(String),
}
