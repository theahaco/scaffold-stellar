use std::{ffi::OsString, path::PathBuf};

use clap::Parser;

use soroban_sdk::xdr::{ScMetaEntry, ScMetaV0};
pub use soroban_spec_tools::contract as contract_spec;
use stellar_cli::{commands::contract::invoke, config, fee};

use crate::contract::NetworkContract;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Path to compiled wasm
    #[arg(long)]
    pub wasm: PathBuf,
    /// Optional author address, if not provided, the default keypair will be used
    #[arg(long, short = 'a')]
    pub author: Option<String>,
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
    #[error(transparent)]
    Config(#[from] config::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        // Read the Wasm file from the path
        let wasm_bytes =
            std::fs::read(&self.wasm).map_err(|_| Error::MissingFileArg(self.wasm.clone()))?;
        let spec =
            contract_spec::Spec::new(&wasm_bytes).map_err(|_| Error::CannotParseContractSpec);

        // Prepare a mutable vector for the base arguments
        let mut args = vec![
            "publish".to_string(),
            "--wasm-file-path".to_string(),
            self.wasm.to_string_lossy().to_string(),
        ];

        // Use `filter_map` to extract relevant metadata and format as arguments
        args.extend(spec?.meta.iter().filter_map(|entry| match entry {
            ScMetaEntry::ScMetaV0(ScMetaV0 { key, val }) => {
                let key_str = key.to_string();
                // TODO add source repository when registry contract supports it
                if key_str == "wasm_name" || key_str == "version" {
                    Some(format!("--{key_str}={val}"))
                } else {
                    None
                }
            }
        }));
        // Use the provided author or the source account
        let author = if let Some(author) = self.author.clone() {
            author
        } else {
            self.config.source_account().await?.to_string()
        };
        args.push(format!("--author={author}"));

        // Pass config and fee to invoke_registry
        self.config
            .invoke_registry(
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
                Some(&self.fee),
            )
            .await?;

        Ok(())
    }
}
