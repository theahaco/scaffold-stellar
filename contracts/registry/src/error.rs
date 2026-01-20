use soroban_sdk::{self, contracterror};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NoSuchWasmPublished = 1,
    /// No such version of the contact has been published
    NoSuchVersion = 2,
    /// Wasm name already claimed
    WasmNameAlreadyTaken = 3,
    /// No such contract deployed
    NoSuchContractDeployed = 4,
    /// Contract already deployed
    AlreadyDeployed = 5,
    /// Failed to redeploy a deployed contract with no coreriff macro
    UpgradeInvokeFailed = 6,
    /// Only Admin is allowed
    AdminOnly = 7,
    /// New version must be greater than the most recent version
    VersionMustBeGreaterThanCurrent = 8,
    /// Invalid name.
    /// Must be at most 64 characters and non-empty;
    /// ascii alphanumeric, '-', or '_';
    /// start with a ascii alphabetic character;
    /// and not be a Rust keyword
    InvalidName = 9,
    /// Must be valid cargo version
    InvalidVersion = 10,
    /// Hash has aleady been published
    HashAlreadyPublished = 11,
}
