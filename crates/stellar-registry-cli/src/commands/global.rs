use std::ops::Deref;

use soroban_rpc as rpc;
use stellar_cli::config::{self};

#[derive(clap::Args, Debug, Clone)]
pub struct Args {
    #[command(flatten)]
    pub config: config::Args,
}

impl Deref for Args {
    type Target = config::Args;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl Args {
    pub fn rpc_client(&self) -> Result<rpc::Client, config::Error> {
        Ok(rpc::Client::new(&self.config.get_network()?.rpc_url)?)
    }
}
