use clap::Parser;

use stellar_cli::{commands::contract::invoke, config};
use stellar_strkey::Contract;

use crate::contract::{NetworkContract, REGISTRY_NAME};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of deployed contract
    pub contract_name: String,

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
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        // Use the network config from flattened args
        let network = self.config.get_network()?;
        let network_passphrase = network.network_passphrase;

        let contract = self.get_contract_id().await?;
        let alias = &self.contract_name;

        // Only create alias mapping, don't fetch wasm here
        self.config
            .locator
            .save_contract_id(&network_passphrase, &contract, alias)?;

        eprintln!("✅ Successfully registered contract alias '{alias}' for {contract}");

        Ok(())
    }

    pub async fn get_contract_id(&self) -> Result<Contract, Error> {
        if self.contract_name == REGISTRY_NAME {
            return Ok(self.config.contract_id()?);
        }
        // Prepare the arguments for invoke_registry
        let slop = ["fetch_contract_id", "--contract-name", &self.contract_name];
        // Use this.config directly
        eprintln!("Fetching contract ID via registry...");
        let raw = self.config.view_registry(&slop).await?;
        let contract_id = raw.trim_matches('"').to_string();
        Ok(contract_id.parse()?)
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
}
