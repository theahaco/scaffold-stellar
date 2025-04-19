use loam_sdk::soroban_sdk::{self, contracterror};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// No such Contract has been published
    NoSuchContractPublished = 1,
    /// No such version of the contact has been published
    NoSuchVersion = 2,
    /// Contract already published
    AlreadyPublished = 3,

    /// No such contract deployed
    NoSuchContractDeployed = 4,

    /// Contract already deployed
    AlreadyDeployed = 5,

    /// Contract already claimed
    AlreadyClaimed = 6,

    /// Failed to initialize contract
    InitFailed = 7,

    /// Failed to redeploy a deployed contract with no coreriff macro
    RedeployDeployedFailed = 8,

    /// Contract doesn't have owner, impossible to perform the operation
    NoOwnerSet = 9,
}
