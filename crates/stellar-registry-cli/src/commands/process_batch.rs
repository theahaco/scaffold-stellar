use clap::Parser;
use stellar_cli::{commands::contract::invoke, config};
use stellar_registry_build::registry::Registry;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Registry channel prefix (e.g. "unverified")
    #[arg(long)]
    pub channel: Option<String>,

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
        let registry = Registry::new(&self.config, self.channel.as_deref()).await?;

        let result = registry
            .as_contract()
            .invoke_with_result(&["process_batch"], self.dry_run)
            .await?;

        eprintln!(
            "{}Processed {result} contracts from batch",
            if self.dry_run { "Dry Run: " } else { "" },
        );
        Ok(())
    }
}
