use clap::Parser;
use stellar_cli::commands::contract::invoke;
use stellar_registry_build::named_registry::PrefixedName;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of published Wasm
    pub wasm_name: PrefixedName,

    /// Version of published Wasm, if not specified, the latest version will be fetched
    #[arg(long)]
    pub version: Option<String>,

    #[command(flatten)]
    pub config: global::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Config(#[from] stellar_cli::config::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let hash = self.fetch_hash().await?;
        println!("{hash}");
        Ok(())
    }

    pub async fn fetch_hash(&self) -> Result<String, Error> {
        let registry = self.wasm_name.registry(&self.config).await?;
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
        Ok(raw.trim_matches('"').to_string())
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {
    use stellar_scaffold_test::RegistryTest;

    #[tokio::test]
    async fn simple() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

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

        let hash = registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .fetch_hash()
            .await
            .unwrap();
        assert!(!hash.is_empty());
        assert!(hash.len() == 64); // 32 bytes hex encoded
    }

    #[tokio::test]
    async fn with_version() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();
        let v2 = registry.hello_wasm_v2();

        // Publish v1
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

        let hash_v1 = registry
            .parse_cmd::<super::Cmd>(&["hello", "--version", "\"0.0.1\""])
            .unwrap()
            .fetch_hash()
            .await
            .unwrap();

        // Publish v2
        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(v2.to_str().unwrap())
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .success();

        let hash_v2 = registry
            .parse_cmd::<super::Cmd>(&["hello", "--version", "\"0.0.2\""])
            .unwrap()
            .fetch_hash()
            .await
            .unwrap();

        // Hashes should be different
        assert_ne!(hash_v1, hash_v2);

        // Without version should return latest (v2)
        let hash_latest = registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .fetch_hash()
            .await
            .unwrap();
        assert_eq!(hash_v2, hash_latest);
    }

    #[tokio::test]
    async fn unverified() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

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

        let hash = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello"])
            .unwrap()
            .fetch_hash()
            .await
            .unwrap();
        assert!(!hash.is_empty());
    }
}
