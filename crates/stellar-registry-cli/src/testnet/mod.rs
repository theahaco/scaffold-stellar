use std::env;

use soroban_sdk::xdr::{Hash, ScAddress};
use stellar_cli::{
    commands::{contract::invoke, NetworkRunnable},
    config::{locator, network, UnresolvedContract},
};

use rpc::Client;
use soroban_rpc as rpc;

const CONTRACT_ID: &str = include_str!("./stellar-registry.json");

pub fn contract_id() -> UnresolvedContract {
    if let Ok(contract_id) = env::var("STELLAR_REGISTRY_CONTRACT_ID") {
        contract_id.parse()
    } else {
        CONTRACT_ID.trim_end().trim_matches('"').to_owned().parse()
    }
    .unwrap()
}

pub fn contract_id_strkey() -> stellar_strkey::Contract {
    contract_id()
        .resolve_contract_id(&locator::Args::default(), &network_passphrase())
        .unwrap()
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
            ..Default::default()
        }
    } else {
        let rpc_url = env::var("SOROBAN_RPC_URL").ok().or_else(|| Some(rpc_url()));
        let network_passphrase = env::var("SOROBAN_NETWORK_PASSPHRASE")
            .ok()
            .or_else(|| Some(network_passphrase()));
        network::Args {
            rpc_url,
            network_passphrase,
            ..Default::default()
        }
    }
}

pub fn build_invoke_cmd(slop: &[&str]) -> invoke::Cmd {
    invoke::Cmd {
        contract_id: contract_id(),
        slop: slop.iter().map(Into::into).collect(),
        config: stellar_cli::config::Args {
            network: network(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub async fn invoke_registry(slop: &[&str]) -> Result<String, invoke::Error> {
    Ok(build_invoke_cmd(slop)
        .run_against_rpc_server(Some(&stellar_cli::commands::global::Args::default()), None)
        .await?
        .into_result()
        .expect("Failed to parse JSON"))
}

pub fn client() -> Result<Client, rpc::Error> {
    Client::new(&rpc_url())
}
