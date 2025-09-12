use std::{ffi::OsString, path::PathBuf};

use clap::Parser;

pub use soroban_spec_tools::contract as contract_spec;
use stellar_cli::{
    commands::contract::invoke,
    config, fee,
    xdr::{ScMetaEntry, ScMetaV0},
};

use crate::contract::NetworkContract;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Path to compiled wasm
    #[arg(long)]
    pub wasm: PathBuf,
    /// Optional author address, if not provided, the default keypair will be used
    #[arg(long, short = 'a')]
    pub author: Option<String>,
    /// Wasm name, if not provided, will try to extract from contract metadata
    #[arg(long)]
    pub wasm_name: Option<String>,
    /// Wasm binary version, if not provided, will try to extract from contract metadata
    #[arg(long)]
    pub binver: Option<String>,
    /// Prepares and simulates publishing with invoking
    #[arg(long)]
    pub dry_run: bool,
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
                match key_str.as_str() {
                    "name" => self
                        .wasm_name
                        .is_none()
                        .then(|| format!("--wasm_name={val}")),
                    "binver" => self.binver.is_none().then(|| format!("--version={val}")),
                    _ => None,
                }
            }
        }));

        // Add wasm_name if specified
        if let Some(ref wasm_name) = self.wasm_name {
            args.push(format!("--wasm_name={wasm_name}"));
        }

        // Add version if specified
        if let Some(ref version) = self.binver {
            args.push(format!("--version={version}"));
        }

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
                self.dry_run,
            )
            .await?;
        eprintln!(
            "{}Succesfully published {args:?}",
            if self.dry_run { "Dry Run: " } else { "" }
        );
        Ok(())
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {

    use stellar_cli::commands::{contract::invoke, global};
    use stellar_scaffold_test::RegistryTest;

    use crate::commands::create_alias;

    #[tokio::test]
    async fn test_run() {
        // Create test environment
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        // Path to the hello world contract WASM
        let wasm_path = test_env
            .cwd
            .join("target/stellar/soroban_hello_world_contract.wasm");

        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .success();

        // Then deploy it
        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .failure();

        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.3")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .success();

        registry
            .parse_cmd::<create_alias::Cmd>(&["registry"])
            .unwrap()
            .run()
            .await
            .unwrap();

        let res = registry
            .parse_cmd::<invoke::Cmd>(&["--id=registry", "--", "current_version", "--wasm-name=hello"])
            .unwrap()
            .invoke(&global::Args::default())
            .await
            .unwrap()
            .into_result()
            .unwrap();
    }
}
