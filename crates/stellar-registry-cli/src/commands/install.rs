use std::path::PathBuf;

use clap::Parser;

use stellar_cli::{
    commands::{
        contract::{fetch, invoke},
        global,
    },
    config::{self, network, UnresolvedContract},
};

use crate::testnet;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of deployed contract
    pub deployed_name: String,
    /// Where to place the Wasm file. Default `<root>/target/soroban/<deployed_name>/index.wasm`
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
    StellarBuild(#[from] stellar_build::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Strkey(#[from] stellar_strkey::DecodeError),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let contract_id = testnet::contract_id();
        let network = testnet::network();
        let out_dir = self.out_dir.as_ref().unwrap();
        let mut out_file = out_dir.join(&self.deployed_name);
        out_file.set_extension("wasm");
        let id_file = out_file.parent().unwrap().join("contract_id.txt");
        let fetch_cmd = fetch::Cmd {
            contract_id,
            out_file: Some(out_file),
            network,
            ..Default::default()
        };
        fetch_cmd.run().await?;
        std::fs::write(id_file, testnet::contract_id_strkey().to_string())?;
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
        cmd.slop = vec!["fetch_contract_id", "--deployed_name", &self.deployed_name]
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        println!("Fetching contract ID...\n{cmd:#?}");
        Ok(cmd.invoke(&global::Args::default()).await?.into_result().unwrap())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_run() {
        std::env::set_var("SOROBAN_NETWORK", "local");
        let cmd = Cmd {
            deployed_name: "stellar_registry".to_owned(),
            out_dir: None,
        };
        let contract_id = testnet::contract_id();
        let network = testnet::network();
        let res = cmd.get_contract_id(&contract_id, &network).await.unwrap();
        println!("{res}");
    }
}
