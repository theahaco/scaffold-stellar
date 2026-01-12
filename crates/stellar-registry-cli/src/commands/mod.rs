use std::str::FromStr;

use clap::{CommandFactory, FromArgMatches, Parser, command};

pub mod create_alias;
pub mod current_version;
pub mod deploy;
pub mod deploy_unnamed;
pub mod download;
pub mod fetch_contract_id;
pub mod fetch_hash;
pub mod global;
pub mod publish;
pub mod publish_hash;
pub mod register_contract;
pub mod upgrade;
pub mod version;

const ABOUT: &str = "Add, manage, and use Wasm packages & named contracts in the Stellar Registry";

#[derive(Parser, Debug)]
#[command(
    name = "stellar-registry",
    about = ABOUT,
    disable_help_subcommand = true,
)]
pub struct Root {
    // #[clap(flatten)]
    // pub global_args: global::Args,
    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Root {
    pub fn new() -> Result<Self, clap::Error> {
        let mut matches = Self::command().get_matches();
        Self::from_arg_matches_mut(&mut matches)
    }

    pub fn from_arg_matches<I, T>(itr: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self::from_arg_matches_mut(&mut Self::command().get_matches_from(itr))
    }
    pub async fn run(&mut self) -> Result<(), Error> {
        match &mut self.cmd {
            Cmd::CurrentVersion(cmd) => cmd.run().await?,
            Cmd::Deploy(deploy) => deploy.run().await?,
            Cmd::DeployUnnamed(cmd) => cmd.run().await?,
            Cmd::Download(cmd) => cmd.run().await?,
            Cmd::FetchContractId(cmd) => cmd.run().await?,
            Cmd::FetchHash(cmd) => cmd.run().await?,
            Cmd::Publish(p) => p.run().await?,
            Cmd::PublishHash(cmd) => cmd.run().await?,
            Cmd::CreateAlias(i) => i.run().await?,
            Cmd::RegisterContract(cmd) => cmd.run().await?,
            Cmd::Version(p) => p.run(),
            Cmd::Upgrade(u) => u.run().await?,
        }
        Ok(())
    }
}

impl FromStr for Root {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_arg_matches(s.split_whitespace())
    }
}

#[derive(Parser, Debug)]
pub enum Cmd {
    /// Create a local `stellar contract alias` from a named registry contract
    CreateAlias(Box<create_alias::Cmd>),
    /// Get the current (latest) version of a published Wasm
    CurrentVersion(Box<current_version::Cmd>),
    /// Deploy a named contract from a published Wasm
    Deploy(Box<deploy::Cmd>),
    /// Deploy a contract from a published Wasm without registering a name
    DeployUnnamed(Box<deploy_unnamed::Cmd>),
    /// Download a Wasm binary, optionally creating a local file
    Download(Box<download::Cmd>),
    /// Look up the contract ID of a deployed contract by name
    FetchContractId(Box<fetch_contract_id::Cmd>),
    /// Fetch the hash of a published Wasm binary
    FetchHash(Box<fetch_hash::Cmd>),
    /// Publish Wasm to registry with package name and semantic version
    Publish(Box<publish::Cmd>),
    /// Publish a Wasm hash (already uploaded) to registry
    PublishHash(Box<publish_hash::Cmd>),
    /// Register an existing contract with a name in the registry
    RegisterContract(Box<register_contract::Cmd>),
    /// Upgrade a contract using a published Wasm
    Upgrade(Box<upgrade::Cmd>),
    /// Version of the scaffold-registry-cli
    Version(version::Cmd),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    CreateAlias(#[from] create_alias::Error),
    #[error(transparent)]
    CurrentVersion(#[from] current_version::Error),
    #[error(transparent)]
    Deploy(#[from] deploy::Error),
    #[error(transparent)]
    DeployUnnamed(#[from] deploy_unnamed::Error),
    #[error(transparent)]
    Download(#[from] download::Error),
    #[error(transparent)]
    FetchContractId(#[from] fetch_contract_id::Error),
    #[error(transparent)]
    FetchHash(#[from] fetch_hash::Error),
    #[error(transparent)]
    Publish(#[from] publish::Error),
    #[error(transparent)]
    PublishHash(#[from] publish_hash::Error),
    #[error(transparent)]
    RegisterContract(#[from] register_contract::Error),
    #[error(transparent)]
    Upgrade(#[from] upgrade::Error),
}
