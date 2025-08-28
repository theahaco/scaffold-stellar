use clap::Parser;
use stellar_cli::{commands::contract::invoke, config};

use crate::contract::NetworkContract;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of contract to upgrade
    #[arg(long)]
    pub contract_name: String,

    /// Name of published Wasm
    #[arg(long)]
    pub wasm_name: String,

    /// Version of published Wasm, if not specified, the latest version will be fetched
    #[arg(long)]
    pub version: Option<String>,

    #[command(flatten)]
    pub config: config::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Strkey(#[from] stellar_strkey::DecodeError),
    #[error(transparent)]
    LocatorConfig(#[from] stellar_cli::config::locator::Error),
    #[error(transparent)]
    Config(#[from] stellar_cli::config::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let mut slop = vec![
            "upgrade_contract",
            "--name",
            &self.contract_name,
            "--wasm-name",
            &self.wasm_name,
        ];
        if let Some(version) = self.version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        self.config.invoke_registry(&slop, None, false).await?;
        let version = if let Some(version) = self.version.as_deref() {
            version.to_string()
        } else {
            self.config
                .invoke_registry(
                    &["current_version", "--wasm-name", &self.wasm_name],
                    None,
                    true,
                )
                .await?
        };
        println!(
            "Upgraded {} to {}@{version}",
            self.contract_name, self.wasm_name
        );
        Ok(())
    }
}
