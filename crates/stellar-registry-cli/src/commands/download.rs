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
        let bytes = self.download_bytes().await?;
        if let Some(file) = self.out_file.as_deref() {
            let mut f = std::fs::File::create(file)?;
            f.write_all(&bytes)?;
        } else {
            std::io::stdout().write_all(&bytes)?;
        }
        Ok(())
    }

    pub async fn download_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut slop = vec!["fetch_hash", "--wasm-name", &self.wasm_name];
        if let Some(version) = self.version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        let raw = self.config.invoke_registry(&slop, None, true).await?;
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

    use std::path::PathBuf;

    use stellar_scaffold_test::RegistryTest;

    #[tokio::test]
    async fn simple() {
        // Create test environment
        let target_dir = PathBuf::from("../../target/stellar")
            .canonicalize()
            .unwrap();
        let v1 = target_dir.join("hello_v1.wasm");

        let registry = RegistryTest::new().await;
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
}
