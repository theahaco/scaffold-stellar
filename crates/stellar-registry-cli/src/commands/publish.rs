use std::{ffi::OsString, path::PathBuf};

use clap::Parser;

use soroban_sdk::xdr::{ScMetaEntry, ScMetaV0};
use stellar_cli::{commands::contract::invoke, config, fee};

pub use soroban_spec_tools::contract as contract_spec;

use crate::testnet::invoke_registry;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Path to compiled wasm
    #[arg(long)]
    pub wasm: PathBuf,
    /// Function name as subcommand, then arguments for that function as `--arg-name value`
    #[arg(last = true, id = "CONTRACT_FN_AND_ARGS")]
    pub slop: Vec<OsString>,

    #[command(flatten)]
    pub config: config::Args,
    #[command(flatten)]
    pub fee: fee::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    SpecTools(#[from] soroban_spec_tools::Error),
    #[error("Cannot parse contract spec")]
    CannotParseContractSpec,
    #[error("Missing file argument {0:#?}")]
    MissingFileArg(PathBuf),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        // Read the Wasm file from the path
        let wasm_bytes =
            std::fs::read(&self.wasm).map_err(|_| Error::MissingFileArg(self.wasm.clone()))?;
        let spec =
            contract_spec::Spec::new(&wasm_bytes).map_err(|_| Error::CannotParseContractSpec);

        // Get out its metadata and set the contract name (wasm_name), version, source_repo
        let mut wasm_name: String = String::new();
        let mut version: String = String::new();
        let mut source_repo: String = String::new();
        for meta_entry in spec?.meta {
            match meta_entry {
                ScMetaEntry::ScMetaV0(ScMetaV0 { key, val }) => {
                    let key = key.to_string();
                    match key.as_str() {
                        "wasm_name" => wasm_name = val.to_string(),
                        "version" => version = val.to_string(),
                        "source_repo" => source_repo = val.to_string(),
                        _ => {}
                    }
                }
            }
        }

        invoke_registry(&[
            "publish",
            "--name",
            &wasm_name,
            "--version",
            &version,
            "--source-repo",
            &source_repo,
        ])
        .await?;

        Ok(())
    }
}
