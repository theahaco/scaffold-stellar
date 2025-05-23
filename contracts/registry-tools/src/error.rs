// use loam_sdk::soroban_sdk::{self, contracterror};

// #[contracterror]
// #[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
// #[repr(u32)]
// pub enum Error {
//     /// No such Contract has been published
//     NoSuchContractPublished = 1,
//     /// No such version of the contact has been published
//     NoSuchVersion = 2,
//     /// Contract already published
//     AlreadyPublished = 3,
//     /// No such contract deployed
//     NoSuchContractDeployed = 4,
//     /// Contract already deployed
//     AlreadyDeployed = 5,
//     /// Contract already claimed
//     AlreadyClaimed = 6,
//     /// Failed to initialize contract
//     InitFailed = 7,
//     /// Failed to redeploy a deployed contract with no coreriff macro
//     RedeployDeployedFailed = 8,
//     /// Contract doesn't have owner, impossible to perform the operation
//     NoOwnerSet = 9,
//     /// Only Admin is allowed
//     AdminOnly = 10,
//     /// New version must be greater than the most recent version
//     VersionMustBeGreaterThanCurrent = 11,
//     /// Invalid name.
//     /// Must be 64 characters or less; ascii alphanumeric or '_'; start with a letter; and not be a Rust keyword
//     InvalidName = 12,
//     /// Invalid Version. Must be valid cargo version
//     InvalidVersion = 13,
// }
