use clap::Parser;
use stellar_cli::commands::contract::invoke;

use crate::{commands::global, contract::NetworkContract};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of contract to upgrade
    #[arg(long)]
    pub contract_name: String,

    /// Name of published Wasm
    #[arg(long)]
    pub wasm_name: String,

    /// Version of published Wasm, if not specified, the latest version will be fetched
    #[arg(long)]
    pub version: Option<String>,

    #[command(flatten)]
    pub config: global::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Strkey(#[from] stellar_strkey::DecodeError),
    #[error(transparent)]
    LocatorConfig(#[from] stellar_cli::config::locator::Error),
    #[error(transparent)]
    Config(#[from] stellar_cli::config::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Upgrade failed: {0:?}")]
    UpgradeFailed(invoke::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let contract_name = &self.contract_name;
        let wasm_name = &self.wasm_name;
        let mut slop = vec![
            "upgrade_contract",
            "--name",
            contract_name,
            "--wasm-name",
            wasm_name,
        ];
        if let Some(version) = self.version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        self.config
            .invoke_registry(&slop, None, false)
            .await
            .map_err(Error::UpgradeFailed)?;
        let version = if let Some(version) = self.version.as_deref() {
            version.to_string()
        } else {
            self.config
                .view_registry(&["current_version", "--wasm-name", wasm_name])
                .await?
        };
        println!("Upgraded {contract_name} to {wasm_name}@{version}",);
        Ok(())
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {

    use stellar_cli::commands::{contract::invoke, global};

    use stellar_scaffold_test::RegistryTest;

    use crate::commands::{create_alias, upgrade};

    #[tokio::test]
    async fn simple_upgrade() {
        // Create test environment
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();
        let v2 = registry.hello_wasm_v2();

        let _test_env = registry.clone().env;

        // Path to the hello world contract WASM

        // First publish the contract
        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(v1.to_str().unwrap())
            .arg("--binver")
            .arg("0.0.1")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .success();

        // Then deploy it
        registry
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg("hello")
            .arg("--wasm-name")
            .arg("hello")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();

        registry
            .parse_cmd::<create_alias::Cmd>(&["hello"])
            .unwrap()
            .run()
            .await
            .unwrap();

        let res = registry
            .parse_cmd::<invoke::Cmd>(&["--id=hello", "--", "hello", "--to=world"])
            .unwrap()
            .invoke(&global::Args::default())
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_eq!(res, r#""world""#);

        // Publish new version
        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(v2.to_str().unwrap())
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg("hello")
            .assert()
            .success();
        println!("Published new version of hello contract");
        registry
            .parse_cmd::<upgrade::Cmd>(&[
                "--contract-name",
                "hello",
                "--wasm-name",
                "hello",
                "--version",
                "\"0.0.2\"",
                "--source=alice",
            ])
            .unwrap()
            .run()
            .await
            .unwrap();

        let res = registry
            .parse_cmd::<invoke::Cmd>(&["--id=hello", "--", "hi", "--to=world"])
            .unwrap()
            .invoke(&global::Args::default())
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_eq!(res, r#""world""#);

        // Upgrade the contract using the old version
        registry
            .parse_cmd::<upgrade::Cmd>(&[
                "--contract-name",
                "hello",
                "--wasm-name",
                "hello",
                "--version",
                "\"0.0.1\"",
                "--source=alice",
            ])
            .unwrap()
            .run()
            .await
            .unwrap();

        let res = registry
            .parse_cmd::<invoke::Cmd>(&["--id=hello", "--", "hello", "--to=world"])
            .unwrap()
            .invoke(&global::Args::default())
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_eq!(res, r#""world""#);

        // Upgrade the contract without specifying version to upgrade to the latest version
        registry
            .parse_cmd::<upgrade::Cmd>(&["--contract-name", "hello", "--wasm-name", "hello"])
            .unwrap()
            .run()
            .await
            .unwrap();

        let res = registry
            .parse_cmd::<invoke::Cmd>(&["--id=hello", "--", "hi", "--to=world"])
            .unwrap()
            .invoke(&global::Args::default())
            .await
            .unwrap()
            .into_result()
            .unwrap();
        assert_eq!(res, r#""world""#);
    }
}
