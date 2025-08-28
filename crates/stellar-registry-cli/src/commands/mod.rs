use std::str::FromStr;

use clap::{command, CommandFactory, FromArgMatches, Parser};

pub mod deploy;
pub mod fetch;
pub mod install;
pub mod publish;
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
            Cmd::Deploy(deploy) => deploy.run().await?,
            Cmd::Fetch(cmd) => cmd.run().await?,
            Cmd::Publish(p) => p.run().await?,
            Cmd::Install(i) => i.run().await?,
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
    /// Deploy a named contract from a published Wasm
    Deploy(Box<deploy::Cmd>),
    /// Fetch a published Wasm
    Fetch(Box<fetch::Cmd>),
    /// Create a local `stellar contract alias` from a named registry contract
    Install(Box<install::Cmd>),
    /// Publish Wasm to registry with package name and semantic version
    Publish(Box<publish::Cmd>),
    /// Version of the scaffold-registry-cli
    Version(version::Cmd),
    /// Upgrade a contract using a published Wasm
    Upgrade(upgrade::Cmd),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Deploy(#[from] deploy::Error),
    #[error(transparent)]
    Fetch(#[from] fetch::Error),
    #[error(transparent)]
    Install(#[from] install::Error),
    #[error(transparent)]
    Publish(#[from] publish::Error),
    #[error(transparent)]
    Upgrade(#[from] upgrade::Error),
}
