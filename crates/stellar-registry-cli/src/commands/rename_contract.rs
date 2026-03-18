use clap::Parser;
use stellar_cli::{commands::contract::invoke, config};
use stellar_registry_build::{named_registry::PrefixedName, registry::Registry};

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Current name of the registered contract
    #[arg(long)]
    pub contract_name: PrefixedName,

    /// New name for the contract
    #[arg(long)]
    pub new_name: String,

    /// Prepares and simulates without invoking
    #[arg(long)]
    pub dry_run: bool,

    #[command(flatten)]
    pub config: global::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    Registry(#[from] stellar_registry_build::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let registry = Registry::new(&self.config, self.contract_name.channel.as_deref()).await?;

        let args = [
            "rename_contract",
            "--old_name",
            &self.contract_name.name,
            "--new_name",
            &self.new_name,
        ];

        registry.as_contract().invoke(&args, self.dry_run).await?;

        eprintln!(
            "{}Successfully renamed '{}' to '{}'",
            if self.dry_run { "Dry Run: " } else { "" },
            self.contract_name.name,
            self.new_name
        );
        Ok(())
    }
}
