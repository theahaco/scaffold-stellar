use std::path::PathBuf;

use clap::Parser;

use stellar_cli::{
    commands::{
        contract::{fetch, invoke},
        global,
    },
    config::{self, locator, network, UnresolvedContract},
};
use stellar_strkey::Contract;

use crate::testnet;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of deployed contract
    pub contract_name: String,
    /// Where to place the Wasm file. Default `<root>/target/stellar/<contract_name>/index.wasm`
    #[arg(long, short = 'o', default_value = "./target/stellar/")]
    pub out_dir: PathBuf,
    /// Directory for storing contract configuration
    #[arg(long, default_value = ".stellar")]
    pub config_dir: PathBuf,
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
    Config(#[from] stellar_cli::config::locator::Error),
}

impl Cmd {
    fn get_config_locator(&self) -> locator::Args {
        locator::Args {
            global: false,
            config_dir: Some(self.config_dir.clone()),
        }
    }

    pub async fn run(&self) -> Result<(), Error> {
        // First fetch the contract ID from the registry
        let network = testnet::network();
        let network_passphrase = network.network_passphrase
        .clone()
        .expect("Network passphrase required");
        let contract_id = self.get_contract_id(&testnet::contract_id(), &network).await?;
        
        let contract = Contract::from_string(&contract_id)?;

        let config_locator = self.get_config_locator();
        config_locator.save_contract_id(
            &network_passphrase,
            &contract,
            &self.contract_name,
        )?;

        // Fetch and save the WASM file
        let mut out_file = self.out_dir.join(&self.contract_name);
        out_file.set_extension("wasm");
        
        let fetch_cmd = fetch::Cmd {
            contract_id: UnresolvedContract::Resolved(contract),
            out_file: Some(out_file.clone()),
            network: network.clone(),
            ..Default::default()
        };
        fetch_cmd.run().await?;
        println!("WASM file saved to: {}", out_file.display());

        println!("✅ Successfully installed contract {}", self.contract_name);
        println!("Contract ID: {}", contract_id);
        
        Ok(())
    }

    pub async fn get_contract_id(
        &self,
        contract_id: &UnresolvedContract,
        network: &network::Args,
    ) -> Result<String, Error> {
        let mut cmd = invoke::Cmd {
            contract_id: contract_id.clone(),
            config: config::Args {
                network: network.clone(),
                ..Default::default()
            },
            is_view: true,
            ..Default::default()
        };
        cmd.slop = vec!["fetch_contract_id", "--contract_name", &self.contract_name]
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        println!("Fetching contract ID...\n{cmd:#?}");
        Ok(cmd
            .invoke(&global::Args::default())
            .await?
            .into_result()
            .unwrap())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_run() {
        std::env::set_var("SOROBAN_NETWORK", "testnet");
        let cmd = Cmd {
            contract_name: "hello".to_owned(),
            out_dir: ".".into(),
            config_dir: ".stellar".into()
        };
        let contract_id = testnet::contract_id();
        let network = testnet::network();
        let res = cmd.get_contract_id(&contract_id, &network).await.unwrap();
        println!("{res}");
    }
}
