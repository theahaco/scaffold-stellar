use std::{io::Write, path::PathBuf};

use clap::Parser;
use stellar_cli::{commands::contract::invoke, config, xdr};

use crate::contract::NetworkContract;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of published Wasm
    pub wasm_name: String,

    /// Version of published Wasm, if not specified, the latest version will be fetched
    #[arg(long)]
    pub version: Option<String>,

    /// Where to write file. default stdout
    #[arg(long, short = 'o')]
    pub out_file: Option<PathBuf>,

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

    #[error(transparent)]
    Rpc(#[from] soroban_rpc::Error),
    #[error(transparent)]
    Xdr(#[from] xdr::Error),
    #[error(transparent)]
    Network(#[from] stellar_cli::config::network::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let mut slop = vec!["fetch_hash", "--wasm-name", &self.wasm_name];
        if let Some(version) = self.version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        let raw = self.config.invoke_registry(&slop, None, true).await?;
        let bytes = stellar_cli::utils::rpc::get_remote_wasm_from_hash(
            &self.config.get_network()?.rpc_client()?,
            &raw.parse()?,
        )
        .await?;
        if let Some(file) = self.out_file.as_deref() {
            let mut f = std::fs::File::create(file)?;
            f.write_all(&bytes)?;
        } else {
            std::io::stdout().write_all(&bytes)?;
        }
        Ok(())
    }
}
