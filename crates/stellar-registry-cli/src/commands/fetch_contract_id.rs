use clap::Parser;
use stellar_cli::commands::contract::invoke;
use stellar_registry_build::named_registry::PrefixedName;
use stellar_strkey::Contract;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of deployed contract. Can use prefix if not using verified registry.
    /// E.g. `unverified/<name>`
    pub contract_name: PrefixedName,

    #[command(flatten)]
    pub config: global::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Config(#[from] stellar_cli::config::Error),
    #[error(transparent)]
    Registry(#[from] stellar_registry_build::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let contract_id = self.fetch_contract_id().await?;
        println!("{contract_id}");
        Ok(())
    }

    pub async fn fetch_contract_id(&self) -> Result<Contract, Error> {
        let registry = self.contract_name.registry(&self.config).await?;
        Ok(registry.fetch_contract_id(&self.contract_name.name).await?)
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

        // Then deploy it
        registry
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg("hello")
            .arg("--wasm-name")
            .arg("hello")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();

        let contract_id = registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .fetch_contract_id()
            .await
            .unwrap();
        assert!(!contract_id.to_string().is_empty());
    }

    #[tokio::test]
    async fn unverified() {
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
            .arg("unverified/hello")
            .assert()
            .success();

        // Then deploy it
        registry
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg("unverified/hello")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();

        let contract_id = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello"])
            .unwrap()
            .fetch_contract_id()
            .await
            .unwrap();
        assert!(!contract_id.to_string().is_empty());
    }
}
