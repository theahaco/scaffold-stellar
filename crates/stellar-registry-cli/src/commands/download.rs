use std::{io::Write, path::PathBuf};

use clap::Parser;
use stellar_cli::{commands::contract::invoke, xdr};
use stellar_registry_build::named_registry::PrefixedName;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of published Wasm
    pub wasm_name: PrefixedName,

    /// Version of published Wasm, if not specified, the latest version will be fetched
    #[arg(long)]
    pub version: Option<String>,

    /// Where to write file. default stdout
    #[arg(long, short = 'o')]
    pub out_file: Option<PathBuf>,

    #[command(flatten)]
    pub config: global::Args,
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
    #[error(transparent)]
    Registry(#[from] stellar_registry_build::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let bytes = self.download_bytes().await?;
        if let Some(file) = self.out_file.as_deref() {
            let parent = file.parent().unwrap();
            std::fs::create_dir_all(parent)?;
            let mut f = std::fs::File::create(file)?;
            f.write_all(&bytes)?;
        } else {
            std::io::stdout().write_all(&bytes)?;
        }
        Ok(())
    }

    pub async fn download_bytes(&self) -> Result<Vec<u8>, Error> {
        let registry = &self.wasm_name.registry(&self.config).await?;
        let mut slop = vec!["fetch_hash", "--wasm-name", &self.wasm_name.name];
        let version = self.version.clone().map(|v| format!("\"{v}\""));
        if let Some(version) = version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        let raw = registry
            .as_contract()
            .invoke_with_result(&slop, None, true)
            .await?;
        let bytes = stellar_cli::utils::rpc::get_remote_wasm_from_hash(
            &self.config.get_network()?.rpc_client()?,
            &raw.trim_matches('"').parse()?,
        )
        .await?;
        Ok(bytes)
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {
    use stellar_scaffold_test::RegistryTest;

    #[tokio::test]
    async fn simple() {
        // Create test environment

        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();
        let _test_env = registry.clone().env;

        // Path to the hello world contract WASM

        // First publish the contract
        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(v1.to_str().unwrap())
            .arg("--binver")
            .arg("0.0.1")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .success();

        let bytes = registry
            .parse_cmd::<crate::commands::download::Cmd>(&["hello"])
            .unwrap()
            .download_bytes()
            .await
            .unwrap();
        let expected = std::fs::read(v1).unwrap();
        assert_eq!(bytes, expected);
    }

    #[tokio::test]
    async fn unverified() {
        // Create test environment

        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();
        let _test_env = registry.clone().env;

        // Path to the hello world contract WASM

        // First publish the contract
        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(v1.to_str().unwrap())
            .arg("--binver")
            .arg("0.0.1")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .assert()
            .success();

        let bytes = registry
            .parse_cmd::<crate::commands::download::Cmd>(&["unverified/hello"])
            .unwrap()
            .download_bytes()
            .await
            .unwrap();
        let expected = std::fs::read(v1).unwrap();
        assert_eq!(bytes, expected);
    }
}
