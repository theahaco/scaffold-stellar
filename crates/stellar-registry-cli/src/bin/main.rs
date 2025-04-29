use clap::{CommandFactory, Parser};

use stellar_registry_cli::{testnet, Root};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv().unwrap_or_default();
    let contract_id = testnet::contract_id_strkey();
    std::env::set_var("STELLAR_CONTRACT_ID", contract_id.to_string());
    // std::env::set_var("SOROBAN_RPC_URL", testnet::rpc_url());
    // std::env::set_var("SOROBAN_NETWORK_PASSPHRASE", testnet::network_passphrase());
    // std::env::remove_var("SOROBAN_NETWORK");
    let mut root = Root::try_parse().unwrap_or_else(|e| {
        let mut cmd = Root::command();
        e.format(&mut cmd).exit();
    });

    if let Err(e) = root.run().await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
