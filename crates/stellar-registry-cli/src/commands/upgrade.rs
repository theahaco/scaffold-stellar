use clap::Parser;
use stellar_cli::commands::contract::invoke;
use stellar_registry_build::named_registry::PrefixedName;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of contract to upgrade.  Can use prefix of not using verified registry.
    /// E.g. `unverified/<name>`
    #[arg(long)]
    pub contract_name: PrefixedName,

    /// Name of published Wasm.  Can use prefix of not using verified registry.
    /// E.g. `unverified/<name>`
    #[arg(long)]
    pub wasm_name: PrefixedName,

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
        let contract_name = &self.contract_name.name;
        let wasm_name = &self.wasm_name.name;

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
        let registry = self.contract_name.registry(&self.config).await?;
        registry
            .as_contract()
            .invoke_with_result(&slop, None, false)
            .await
            .map_err(Error::UpgradeFailed)?;
        let version = if let Some(version) = self.version.as_deref() {
            version.to_string()
        } else {
            registry
                .as_contract()
                .invoke_with_result(&["current_version", "--wasm-name", wasm_name], None, true)
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

    #[tokio::test]
    async fn unverified() {
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
            .arg("unverified/hello")
            .assert()
            .success();

        // Then deploy it
        registry
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg("unverified/hello")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();

        registry
            .parse_cmd::<create_alias::Cmd>(&["unverified/hello"])
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
            .arg("unverified/hello")
            .assert()
            .success();
        println!("Published new version of hello contract");
        registry
            .parse_cmd::<upgrade::Cmd>(&[
                "--contract-name",
                "unverified/hello",
                "--wasm-name",
                "unverified/hello",
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
                "unverified/hello",
                "--wasm-name",
                "unverified/hello",
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
            .parse_cmd::<upgrade::Cmd>(&[
                "--contract-name",
                "unverified/hello",
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
