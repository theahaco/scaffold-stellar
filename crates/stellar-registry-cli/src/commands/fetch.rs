use std::{io::Write, path::PathBuf};

use clap::Parser;
use stellar_cli::{commands::contract::invoke, config};

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
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let mut slop = vec!["fetch_hash", "--wasm-name", &self.wasm_name];
        if let Some(version) = self.version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        let raw = self.config.invoke_registry(&slop, None, true).await?;
        if let Some(file) = self.out_file.as_deref() {
            let mut f = std::fs::File::create(file)?;
            f.write_all(raw.as_bytes())?;
        } else {
            println!("{raw}");
        }
        Ok(())
    }
}
