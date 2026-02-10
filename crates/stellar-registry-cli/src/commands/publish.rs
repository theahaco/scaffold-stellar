use std::{ffi::OsString, path::PathBuf};

use clap::{Args, Parser};

pub use soroban_spec_tools::contract as contract_spec;
use stellar_cli::{
    commands::contract::invoke,
    config,
    xdr::{ScMetaEntry, ScMetaV0},
};
use stellar_registry_build::{named_registry::PrefixedName, registry::Registry};

use crate::{commands::global, github::Fetcher};

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct WasmArgs {
    /// Path to compiled wasm
    #[arg(long)]
    pub wasm: Option<PathBuf>,
    /// Optionally can provide a github repo (<org>/<repo>) which hosts a contract with attestation
    #[arg(long)]
    pub from_github: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    #[command(flatten)]
    pub wasm_args: WasmArgs,
    /// Optional author address, if not provided, the default keypair will be used
    #[arg(long, short = 'a')]
    pub author: Option<String>,
    /// Wasm name, if not provided, will try to extract from contract metadata
    #[arg(long, requires = "from_github")]
    pub wasm_name: Option<PrefixedName>,
    /// Wasm binary version, if not provided, will try to extract from contract metadata
    #[arg(long, requires = "from_github")]
    pub binver: Option<String>,
    /// Prepares and simulates publishing with invoking
    #[arg(long)]
    pub dry_run: bool,
    /// Function name as subcommand, then arguments for that function as `--arg-name value`
    #[arg(last = true, id = "CONTRACT_FN_AND_ARGS")]
    pub slop: Vec<OsString>,
    #[command(flatten)]
    pub config: global::Args,
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
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("Invalid arguments: --wasm or --github required")]
    InvalidWasmArgs,
    #[error("--github requires --binver")]
    BinverMissing,
    #[error("--github requires --wasm-name")]
    WasmNameMissing,
    #[error(transparent)]
    Registry(#[from] stellar_registry_build::Error),
}

impl Cmd {
    pub async fn get_wasm_bytes(&self) -> Result<Vec<u8>, Error> {
        if let Some(github) = &self.wasm_args.from_github {
            Ok(Fetcher::new(
                github,
                &self.wasm_name.as_ref().ok_or(Error::WasmNameMissing)?.name,
                self.binver.as_ref().ok_or(Error::BinverMissing)?,
            )
            .fetch()
            .await?)
        } else if let Some(wasm) = &self.wasm_args.wasm {
            std::fs::read(wasm).map_err(|_| Error::MissingFileArg(wasm.clone()))
        } else {
            Err(Error::InvalidWasmArgs)
        }
    }

    pub async fn run(&self) -> Result<(), Error> {
        // Read the Wasm file from the path
        let wasm_bytes = self.get_wasm_bytes().await?;
        let spec =
            contract_spec::Spec::new(&wasm_bytes).map_err(|_| Error::CannotParseContractSpec)?;
        // Prepare a mutable vector for the base arguments
        let mut args = vec![
            "publish".to_string(),
            "--wasm".to_string(),
            hex::encode(wasm_bytes),
        ];

        // Use `filter_map` to extract relevant metadata and format as arguments
        args.extend(spec.meta.iter().filter_map(|entry| match entry {
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
    use stellar_scaffold_test::{AssertExt, RegistryTest};

    #[tokio::test]
    async fn verified() {
        // Create test environment
        let registry = RegistryTest::new().await;

        // Path to the hello world contract WASM
        let wasm_path = registry.hello_wasm_v1();
        let wasm_path_two = registry.hello_wasm_v2();

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

        // Then publish with different wasm same version
        let stderr = registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path_two)
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .failure()
            .stderr_as_str();
        assert!(stderr.contains("Error(Contract, #8)"));

        // Different version same wasm
        let stderr = registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.3")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .failure()
            .stderr_as_str();
        assert!(stderr.contains("Error(Contract, #11)"));

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

    #[tokio::test]
    async fn unverified() {
        // Create test environment
        let registry = RegistryTest::new().await;

        // Path to the hello world contract WASM
        let wasm_path = registry.hello_wasm_v1();
        let wasm_path_two = registry.hello_wasm_v2();

        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .assert()
            .success();

        // publish new wasm with same name
        let stderr = registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path_two)
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .assert()
            .failure()
            .stderr_as_str();
        assert!(stderr.contains("Error(Contract, #8)"));

        let stderr = registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.3")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .assert()
            .failure()
            .stderr_as_str();

        assert!(stderr.contains("Error(Contract, #11)"));

        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path_two)
            .arg("--binver")
            .arg("0.0.3")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .assert()
            .success();
    }

    #[tokio::test]
    async fn github() {
        // Create test environment
        let registry = RegistryTest::new().await;

        registry
            .registry_cli("publish")
            .arg("--from-github")
            .arg("theahaco/scaffold-stellar")
            .arg("--binver")
            .arg("0.3.1")
            .arg("--wasm-name")
            .arg("registry")
            .assert()
            .success();

        assert_eq!(
            "cbd955f16a026c6658b3f28bc205240db580424433d3ac85ccc55062f015add6",
            &registry
                .registry_cli("fetch-hash")
                .arg("registry")
                .assert()
                .success()
                .stdout_as_str()
        );
    }
}
