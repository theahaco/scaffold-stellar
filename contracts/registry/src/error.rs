#[soroban_sdk_tools::scerr]
pub enum Error {
    NoSuchWasmPublished,
    /// No such version of the contact has been published
    NoSuchVersion,
    /// Wasm name already claimed
    WasmNameAlreadyTaken,
    /// No such contract deployed
    NoSuchContractDeployed,
    /// Contract already deployed
    AlreadyDeployed,
    /// Failed to upgrade a contract
    UpgradeInvokeFailed,
    /// Only Admin is allowed
    AdminOnly,
    /// New version must be greater than the most recent version
    VersionMustBeGreaterThanCurrent,
    /// Invalid name.
    /// Must be at most 64 characters and non-empty;
    /// ascii alphanumeric, '-', or '_';
    /// start with a ascii alphabetic character;
    /// and not be a Rust keyword
    InvalidName,
    /// Must be valid cargo version
    InvalidVersion,
    /// Hash has aleady been published
    HashAlreadyPublished,
    /// Root registry requires manager when deploying
    ManagerRequired,
    /// No pending batch entries to process
    NoPendingBatch,
    /// Caller is not the contract owner
    NotContractOwner,
    /// Batch entry missing from temporary storage (likely expired)
    BatchEntryExpired,
    /// Given "contract ID" appears to be a G-address, not a contract ID
    AccountAddressNotValid,
    /// Given contract ID does not exist on this network
    ContractIdAddressDoesNotExist,
    /// Invoking contract's function has failed
    ProxyInvocationFailed,
    /// Contract to be invoked is compromised
    ProxyContractCompromised,
}
