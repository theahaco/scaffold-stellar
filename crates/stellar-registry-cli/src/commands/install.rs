use std::path::PathBuf;

use clap::Parser;

use smartdeploy_build::{target_dir, wasm_location};
use soroban_cli::commands::{
    contract::{fetch, invoke},
    global, network,
};

use crate::testnet;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of deployed contract
    pub deployed_name: String,
    /// Where to place the Wasm file. Default `<root>/target/smartdeploy/<deployed_name>/index.wasm`
    #[arg(long, short = 'o')]
    pub out_dir: Option<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Fetch(#[from] fetch::Error),
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    SmartdeployBuild(#[from] smartdeploy_build::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Strkey(#[from] stellar_strkey::DecodeError),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let contract_id = testnet::contract_id();
        let network = testnet::network();
        let id = self.get_contract_id(&contract_id, &network).await?;
        let contract_id = id.trim_matches('"');
        let out_dir = if let Some(out_dir) = self.out_dir.clone() {
            out_dir
        } else {
            target_dir()?
        };
        let out_file = wasm_location(&self.deployed_name, Some(&out_dir))?;
        let id_file = out_file.parent().unwrap().join("contract_id.txt");
        let fetch_cmd = fetch::Cmd {
            contract_id: contract_id.to_owned(),
            out_file: Some(out_file),
            network,
            ..Default::default()
        };
        fetch_cmd.run().await?;
        std::fs::write(id_file, contract_id)?;
        Ok(())
    }

    pub async fn get_contract_id(
        &self,
        smartdeploy_contract_id: &str,
        network: &network::Args,
    ) -> Result<String, Error> {
        let mut cmd = invoke::Cmd {
            contract_id: smartdeploy_contract_id.to_string(),
            config: soroban_cli::commands::config::Args {
                network: network.clone(),
                ..Default::default()
            },
            is_view: true,
            ..Default::default()
        };
        cmd.slop = vec!["fetch_contract_id", "--deployed_name", &self.deployed_name]
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        println!("Fetching contract ID...\n{cmd:#?}");
        Ok(cmd.invoke(&global::Args::default()).await?)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_run() {
        std::env::set_var("SOROBAN_NETWORK", "local");
        let cmd = Cmd {
            deployed_name: "smartdeploy".to_owned(),
            out_dir: None,
        };
        let contract_id = testnet::contract_id();
        let network = testnet::network();
        let res = cmd.get_contract_id(&contract_id, &network).await.unwrap();
        println!("{res}");
    }
}
