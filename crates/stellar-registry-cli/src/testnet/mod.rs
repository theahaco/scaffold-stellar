use std::env;

use soroban_sdk::xdr::{Hash, ScAddress};
use stellar_cli::{
    commands::{contract::invoke, NetworkRunnable},
    config::{locator, network, UnresolvedContract},
};

use rpc::Client;
use soroban_rpc as rpc;

pub fn contract_id() -> UnresolvedContract {
    if let Ok(contract_id) = env::var("STELLAR_REGISTRY_CONTRACT_ID") {
        contract_id.parse()
    } else {
        super::contract::contract_id(&network_passphrase())
            .to_string()
            .parse()
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
    env::var("STELLAR_RPC_URL")
        .unwrap_or_else(|_| "https://soroban-testnet.stellar.org:443".to_owned())
}

pub fn network_passphrase() -> String {
    env::var("STELLAR_NETWORK_PASSPHRASE")
        .unwrap_or_else(|_| "Test SDF Network ; September 2015".to_owned())
}

pub fn network() -> network::Args {
    if let Ok(network) = env::var("STELLAR_NETWORK") {
        network::Args {
            network: Some(network),
            ..Default::default()
        }
    } else {
        network::Args {
            rpc_url: Some(rpc_url()),
            network_passphrase: Some(network_passphrase()),
            ..Default::default()
        }
    }
}

pub fn build_invoke_cmd(
    slop: &[&str],
    config: &stellar_cli::config::Args,
    fee: &stellar_cli::fee::Args,
) -> invoke::Cmd {
    invoke::Cmd {
        contract_id: contract_id(),
        slop: slop.iter().map(Into::into).collect(),
        config: config.clone(),
        fee: fee.clone(),
        ..Default::default()
    }
}

pub async fn invoke_registry(
    slop: &[&str],
    config: &stellar_cli::config::Args,
    fee: &stellar_cli::fee::Args,
) -> Result<String, invoke::Error> {
    Ok(build_invoke_cmd(slop, config, fee)
        .run_against_rpc_server(Some(&stellar_cli::commands::global::Args::default()), None)
        .await?
        .into_result()
        .expect("Failed to parse JSON"))
}

pub fn client() -> Result<Client, rpc::Error> {
    Client::new(&rpc_url())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_contract_id() {
        let contract_id = super::contract_id_strkey();
        assert_eq!(
            contract_id.to_string(),
            "CAUZNARBNJOFLYURIINDQDRUOJWVGR3VOH6QWFNQEPZWRQJFCYUHSJU7".to_string()
        );
    }
}
