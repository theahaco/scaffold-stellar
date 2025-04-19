use clap::Parser;

use smartdeploy_build::contract_id;
use soroban_cli::commands::{contract::invoke, global};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    pub deployed_name: String,
    #[command(flatten)]
    pub call: invoke::Cmd,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    SmartdeployBuild(#[from] smartdeploy_build::Error),
    #[error(transparent)]
    Install(#[from] super::install::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let id = self.contract_id().await?;
        let mut contract = self.call.clone();
        contract.contract_id = id.trim_matches('"').to_string();
        contract.run(&global::Args::default()).await?;
        Ok(())
    }

    pub async fn contract_id(&self) -> Result<String, Error> {
        let res = contract_id(&self.deployed_name, None);
        Ok(
            if let Err(smartdeploy_build::Error::MissingContractId(_)) = &res {
                super::install::Cmd {
                    deployed_name: self.deployed_name.clone(),
                    out_dir: None,
                }
                .run()
                .await?;
                contract_id(&self.deployed_name, None)
            } else {
                res
            }?,
        )
    }
}
