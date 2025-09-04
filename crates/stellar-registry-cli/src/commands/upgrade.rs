use clap::Parser;
use stellar_cli::{commands::contract::invoke, config};

use crate::contract::NetworkContract;

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
    pub config: config::Args,
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
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let mut slop = vec![
            "upgrade_contract",
            "--name",
            &self.contract_name,
            "--wasm-name",
            &self.wasm_name,
        ];
        if let Some(version) = self.version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        self.config.invoke_registry(&slop, None, false).await?;
        let version = if let Some(version) = self.version.as_deref() {
            version.to_string()
        } else {
            self.config
                .invoke_registry(
                    &["current_version", "--wasm-name", &self.wasm_name],
                    None,
                    true,
                )
                .await?
        };
        println!(
            "Upgraded {} to {}@{version}",
            self.contract_name, self.wasm_name
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use stellar_cli::commands::{contract::invoke, global};

    use stellar_scaffold_test::RegistryTest;

    use crate::commands::{create_alias, upgrade};

    #[tokio::test]
    async fn simple_upgrade() {
        // Create test environment
        let target_dir = PathBuf::from("../../target/stellar")
            .canonicalize()
            .unwrap();
        let v1 = target_dir.join("hello_v1.wasm");
        let v2 = target_dir.join("hello_v2.wasm");

        let registry = RegistryTest::new().await;
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

        registry
            .parse_cmd::<upgrade::Cmd>(&[
                "--contract-name",
                "hello",
                "--wasm-name",
                "hello",
                "--version",
                "0.0.2",
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
                "0.0.1",
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
            .parse_cmd::<upgrade::Cmd>(&[
                "--contract-name",
                "hello",
                "--wasm-name",
                "hello",
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

    }
}
