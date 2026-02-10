use clap::Parser;
use stellar_cli::{commands::contract::invoke, config, fee};
use stellar_registry_build::{named_registry::PrefixedName, registry::Registry};

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Wasm hash to publish (hex-encoded 32 bytes)
    #[arg(long)]
    pub wasm_hash: String,

    /// Wasm name
    #[arg(long)]
    pub wasm_name: PrefixedName,

    /// Version string (e.g. "0.0.1")
    #[arg(long)]
    pub version: String,

    /// Optional author address, if not provided, the default keypair will be used
    #[arg(long, short = 'a')]
    pub author: Option<String>,

    /// Prepares and simulates publishing without invoking
    #[arg(long)]
    pub dry_run: bool,

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
    Config(#[from] config::Error),
    #[error(transparent)]
    Registry(#[from] stellar_registry_build::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let author = if let Some(author) = self.author.clone() {
            author
        } else {
            self.config.source_account().await?.to_string()
        };

        let args = [
            "publish_hash",
            "--wasm_name",
            &self.wasm_name.name,
            "--author",
            &author,
            "--wasm_hash",
            &self.wasm_hash,
            "--version",
            &self.version,
        ];

        let registry = Registry::new(&self.config, self.wasm_name.channel.as_deref()).await?;

        registry
            .as_contract()
            .invoke(&args, Some(&self.fee), self.dry_run)
            .await?;

        eprintln!(
            "{}Successfully published hash {} as {}@{}",
            if self.dry_run { "Dry Run: " } else { "" },
            self.wasm_hash,
            self.wasm_name.name,
            self.version
        );
        Ok(())
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {
    use stellar_cli::commands::{NetworkRunnable, contract::upload};
    use stellar_scaffold_test::RegistryTest;

    #[tokio::test]
    async fn simple() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

        // First upload the wasm to get a hash
        let hash = registry
            .parse_cmd::<upload::Cmd>(&[
                "--wasm",
                v1.to_str().unwrap(),
                "--source",
                "alice",
                "--fee=10000000",
            ])
            .unwrap()
            .run_against_rpc_server(None, None)
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .to_string();

        // Now publish using the hash
        registry
            .registry_cli("publish-hash")
            .arg("--wasm-hash")
            .arg(&hash)
            .arg("--wasm-name")
            .arg("hello")
            .arg("--version")
            .arg("0.0.1")
            .assert()
            .success();

        // Verify the hash was published
        let fetched_hash = registry
            .parse_cmd::<crate::commands::fetch_hash::Cmd>(&["hello"])
            .unwrap()
            .fetch_hash()
            .await
            .unwrap();
        assert_eq!(hash, fetched_hash);
    }

    #[tokio::test]
    async fn unverified() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

        // First upload the wasm to get a hash
        let hash = registry
            .parse_cmd::<upload::Cmd>(&[
                "--wasm",
                v1.to_str().unwrap(),
                "--source",
                "alice",
                "--fee=10000000",
            ])
            .unwrap()
            .run_against_rpc_server(None, None)
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .to_string();

        // Now publish using the hash to unverified registry
        registry
            .registry_cli("publish-hash")
            .arg("--wasm-hash")
            .arg(&hash)
            .arg("--wasm-name")
            .arg("unverified/hello")
            .arg("--version")
            .arg("0.0.1")
            .assert()
            .success();

        // Verify the hash was published
        let fetched_hash = registry
            .parse_cmd::<crate::commands::fetch_hash::Cmd>(&["unverified/hello"])
            .unwrap()
            .fetch_hash()
            .await
            .unwrap();
        assert_eq!(hash, fetched_hash);
    }
}
