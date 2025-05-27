use clap::Parser;

use stellar_cli::{
    commands::contract::{fetch, invoke},
    config,
};
use stellar_strkey::Contract;

use crate::contract::NetworkContract;

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
    Fetch(#[from] fetch::Error),
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    StellarBuild(#[from] stellar_build::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
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

        // Only create alias mapping, don't fetch wasm here
        self.config.locator.save_contract_id(
            &network_passphrase,
            &contract,
            &self.contract_name,
        )?;

        eprintln!(
            "✅ Successfully registered contract alias '{}'",
            self.contract_name
        );
        eprintln!("Contract ID: {:?}", contract.to_string());

        Ok(())
    }

    pub async fn get_contract_id(&self) -> Result<Contract, Error> {
        // Prepare the arguments for invoke_registry
        let slop = vec!["fetch_contract_id", "--contract-name", &self.contract_name];

        // Use this.config directly
        eprintln!("Fetching contract ID via registry...");
        let raw = self.config.invoke_registry(&slop, None, true).await?;

        let contract_id = raw.trim_matches('"').to_string();
        Ok(contract_id.parse()?)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "integration-tests")]
    #[tokio::test]
    async fn test_run() {
        use super::*;
        use std::env;
        use stellar_cli::config::{locator, network};
        use stellar_scaffold_test::RegistryTest;
        // Create test environment
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        // Set environment variables for testnet configuration
        env::set_var("STELLAR_RPC_URL", "http://localhost:8000/soroban/rpc");
        env::set_var("STELLAR_ACCOUNT", "alice");
        env::set_var(
            "STELLAR_NETWORK_PASSPHRASE",
            "Standalone Network ; February 2017",
        );
        env::set_var("STELLAR_REGISTRY_CONTRACT_ID", &registry.registry_address);

        // Path to the hello world contract WASM
        let wasm_path = test_env
            .cwd
            .join("target/stellar/soroban_hello_world_contract.wasm");

        // First publish the contract
        registry
            .clone()
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .assert()
            .success();

        // Then deploy it
        registry
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg("hello")
            .arg("--wasm-name")
            .arg("soroban-hello-world-contract")
            .assert()
            .success();

        // Create test command for install
        let cmd = Cmd {
            contract_name: "hello".to_owned(),
            config: config::Args {
                locator: locator::Args {
                    global: false,
                    config_dir: Some(test_env.cwd.to_str().unwrap().into()),
                },
                network: network::Args {
                    rpc_url: Some("http://localhost:8000/soroban/rpc".to_string()),
                    network_passphrase: Some("Standalone Network ; February 2017".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        // Run the install command
        cmd.run().await.unwrap();
        assert!(test_env.cwd.join(".stellar").exists());
    }
}
