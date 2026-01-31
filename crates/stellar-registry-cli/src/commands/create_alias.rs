use clap::Parser;

use stellar_cli::commands::contract::invoke;
use stellar_registry_build::named_registry::PrefixedName;
use stellar_strkey::Contract;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of deployed contract. Can use prefix of not using verified registry.
    /// E.g. `unverified/<name>`
    pub contract: PrefixedName,

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
    Registry(#[from] stellar_registry_build::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let network_passphrase = self.config.get_network()?.network_passphrase;
        let alias = &self.contract.name;
        let contract = self.get_contract_id().await?;
        // Only create alias mapping, don't fetch wasm here
        self.config
            .locator
            .save_contract_id(&network_passphrase, &contract, alias)?;
        eprintln!("✅ Successfully registered contract alias '{alias}' for {contract}");
        Ok(())
    }

    pub async fn get_contract_id(&self) -> Result<Contract, Error> {
        let registry = &self.contract.registry(&self.config).await?;
        eprintln!("Fetching contract ID via registry...");
        Ok(registry.fetch_contract_id(&self.contract.name).await?)
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
        let test_env = registry.clone().env;

        // Path to the hello world contract WASM
        let wasm_path = registry.hello_wasm_v1();

        // First publish the contract
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
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg("hello")
            .arg("--wasm-name")
            .arg("hello")
            .arg("--version")
            .arg("0.0.2")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();

        // Create test command for install
        let cmd = registry.parse_cmd::<super::Cmd>(&["hello"]).unwrap();

        // Run the install command
        cmd.run().await.unwrap();
        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/hello.json")
                .exists()
        );
    }

    #[tokio::test]
    async fn unverified() {
        // Create test environment
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        // Path to the hello world contract WASM
        let wasm_path = registry.hello_wasm_v1();

        // First publish the contract
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

        // Then deploy it
        registry
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg("unverified/hello")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .arg("--version")
            .arg("0.0.2")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();

        // Create test command for install
        let cmd = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello"])
            .unwrap();

        // Run the install command
        cmd.run().await.unwrap();
        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/hello.json")
                .exists()
        );
    }
}
