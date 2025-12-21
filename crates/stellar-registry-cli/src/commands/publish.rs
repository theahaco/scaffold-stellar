use std::{ffi::OsString, path::PathBuf};

use clap::Parser;

pub use soroban_spec_tools::contract as contract_spec;
use stellar_cli::{
    commands::contract::invoke,
    config, fee,
    xdr::{ScMetaEntry, ScMetaV0},
};
use stellar_registry_build::{named_registry::PrefixedName, registry::Registry};

use crate::commands::global;

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
    pub wasm_name: Option<PrefixedName>,
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
    pub config: global::Args,
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
                    "binver" => self
                        .binver
                        .is_none()
                        .then(|| format!("--version=\"{val}\"")),
                    _ => None,
                }
            }
        }));

        // Add wasm_name if specified
        if let Some(PrefixedName { name, .. }) = self.wasm_name.as_ref() {
            args.push(format!("--wasm_name={name}"));
        }

        // Add version if specified
        if let Some(ref version) = self.binver {
            args.push(format!("--version=\"{version}\""));
        }

        // Use the provided author or the source account
        let author = if let Some(author) = self.author.clone() {
            author
        } else {
            self.config.source_account().await?.to_string()
        };
        args.push(format!("--author={author}"));
        let registry = Registry::new(
            &self.config,
            self.wasm_name.as_ref().and_then(|p| p.channel.as_deref()),
        )
        .await?;
        registry
            .as_contract()
            .invoke(
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
    use stellar_scaffold_test::RegistryTest;

    #[tokio::test]
    async fn test_run() {
        // Create test environment
        let registry = RegistryTest::new().await;

        // Path to the hello world contract WASM
        let wasm_path = registry.hello_wasm_v1();

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

        let output = registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.3")
            .arg("--wasm-name")
            .arg("hello")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Error(Contract, #11)"));

        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&registry.hello_wasm_v2())
            .arg("--binver")
            .arg("0.0.3")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .success();
    }
}
