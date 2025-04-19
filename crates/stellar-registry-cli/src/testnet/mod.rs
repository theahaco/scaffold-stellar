use std::env;

use loam_sdk::soroban_sdk::xdr::{Hash, ScAddress};
use soroban_cli::commands::{contract::invoke, network, NetworkRunnable};

use rpc::Client;
use soroban_rpc as rpc;

const CONTRACT_ID: &str = include_str!("./smartdeploy.json");

pub fn contract_id() -> String {
    if let Ok(contract_id) = env::var("SMARTDEPLOY_CONTRACT_ID") {
        contract_id
    } else {
        CONTRACT_ID.trim_end().trim_matches('"').to_owned()
    }
}

pub fn contract_id_strkey() -> stellar_strkey::Contract {
    stellar_strkey::Contract::from_string(&contract_id()).unwrap()
}

pub fn contract_address() -> ScAddress {
    ScAddress::Contract(Hash(contract_id_strkey().0))
}

pub fn rpc_url() -> String {
    "https://soroban-testnet.stellar.org:443".to_owned()
}

pub fn network_passphrase() -> String {
    "Test SDF Network ; September 2015".to_owned()
}

pub fn network() -> network::Args {
    if let Ok(network) = env::var("SOROBAN_NETWORK") {
        network::Args {
            network: Some(network),
            rpc_url: None,
            network_passphrase: None,
        }
    } else {
        let rpc_url = env::var("SOROBAN_RPC_URL").ok().or_else(|| Some(rpc_url()));
        let network_passphrase = env::var("SOROBAN_NETWORK_PASSPHRASE")
            .ok()
            .or_else(|| Some(network_passphrase()));
        network::Args {
            network: None,
            rpc_url,
            network_passphrase,
        }
    }
}

pub fn build_invoke_cmd(slop: &[&str]) -> invoke::Cmd {
    invoke::Cmd {
        contract_id: contract_id(),
        slop: slop.iter().map(Into::into).collect(),
        config: soroban_cli::commands::config::Args {
            network: network(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub async fn invoke_smartdeploy(slop: &[&str]) -> Result<String, invoke::Error> {
    build_invoke_cmd(slop)
        .run_against_rpc_server(Some(&soroban_cli::commands::global::Args::default()), None)
        .await
}

pub fn client() -> Result<Client, rpc::Error> {
    Client::new(&rpc_url())
}
