use clap::Parser;
use stellar_cli::{commands::contract::invoke, config, fee};
use stellar_registry_build::{named_registry::PrefixedName, registry::Registry};

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name to register for the contract. Can use prefix if not using verified registry.
    /// E.g. `unverified/<name>`
    #[arg(long)]
    pub contract_name: PrefixedName,

    /// Contract address to register
    #[arg(long)]
    pub contract_address: String,

    /// Owner of the contract registration
    #[arg(long)]
    pub owner: Option<String>,

    /// Prepares and simulates without invoking
    #[arg(long)]
    pub dry_run: bool,

    #[command(flatten)]
    pub config: global::Args,

    #[command(flatten)]
    pub fee: fee::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let owner = if let Some(owner) = self.owner.clone() {
            owner
        } else {
            self.config.source_account().await?.to_string()
        };

        let args = vec![
            "register_contract".to_string(),
            format!("--contract_name={}", self.contract_name.name),
            format!("--contract_address={}", self.contract_address),
            format!("--owner={owner}"),
        ];

        let registry = Registry::new(&self.config, self.contract_name.channel.as_deref()).await?;

        registry
            .as_contract()
            .invoke(
                &args.iter().map(String::as_str).collect::<Vec<_>>(),
                Some(&self.fee),
                self.dry_run,
            )
            .await?;

        eprintln!(
            "{}Successfully registered contract '{}' at {}",
            if self.dry_run { "Dry Run: " } else { "" },
            self.contract_name.name,
            self.contract_address
        );
        Ok(())
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {
    use stellar_cli::commands::{NetworkRunnable, contract::deploy::wasm};
    use stellar_scaffold_test::RegistryTest;

    #[tokio::test]
    async fn simple() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

        // Deploy a contract directly (not through registry)
        let contract_id = registry
            .parse_cmd::<wasm::Cmd>(&[
                "--wasm",
                v1.to_str().unwrap(),
                "--source",
                "alice",
                "--fee=10000000",
                "--",
                "--admin=alice",
            ])
            .unwrap()
            .run_against_rpc_server(None, None)
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .to_string();

        // Now register it in the registry
        registry
            .registry_cli("register-contract")
            .arg("--contract-name")
            .arg("my-hello")
            .arg("--contract-address")
            .arg(&contract_id)
            .assert()
            .success();

        // Verify we can fetch the contract ID
        let fetched_id = registry
            .parse_cmd::<crate::commands::fetch_contract_id::Cmd>(&["my-hello"])
            .unwrap()
            .fetch_contract_id()
            .await
            .unwrap();
        assert_eq!(contract_id, fetched_id.to_string());
    }

    #[tokio::test]
    async fn unverified() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

        // Deploy a contract directly (not through registry)
        let contract_id = registry
            .parse_cmd::<wasm::Cmd>(&[
                "--wasm",
                v1.to_str().unwrap(),
                "--source",
                "alice",
                "--fee=10000000",
                "--",
                "--admin=alice",
            ])
            .unwrap()
            .run_against_rpc_server(None, None)
            .await
            .unwrap()
            .into_result()
            .unwrap()
            .to_string();

        // Now register it in the unverified registry
        registry
            .registry_cli("register-contract")
            .arg("--contract-name")
            .arg("unverified/my-hello")
            .arg("--contract-address")
            .arg(&contract_id)
            .assert()
            .success();

        // Verify we can fetch the contract ID
        let fetched_id = registry
            .parse_cmd::<crate::commands::fetch_contract_id::Cmd>(&["unverified/my-hello"])
            .unwrap()
            .fetch_contract_id()
            .await
            .unwrap();
        assert_eq!(contract_id, fetched_id.to_string());
    }
}
