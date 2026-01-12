use clap::Parser;
use stellar_cli::commands::contract::invoke;
use stellar_registry_build::named_registry::PrefixedName;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of published Wasm
    pub wasm_name: PrefixedName,

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
        let version = self.current_version().await?;
        println!("{version}");
        Ok(())
    }

    pub async fn current_version(&self) -> Result<String, Error> {
        let registry = self.wasm_name.registry(&self.config).await?;
        let slop = ["current_version", "--wasm-name", &self.wasm_name.name];
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

        let version = registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .current_version()
            .await
            .unwrap();
        assert_eq!(version, "0.0.1");
    }

    #[tokio::test]
    async fn multiple_versions() {
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

        let version = registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .current_version()
            .await
            .unwrap();
        assert_eq!(version, "0.0.1");

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

        let version = registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .current_version()
            .await
            .unwrap();
        assert_eq!(version, "0.0.2");
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

        let version = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello"])
            .unwrap()
            .current_version()
            .await
            .unwrap();
        assert_eq!(version, "0.0.1");
    }
}
