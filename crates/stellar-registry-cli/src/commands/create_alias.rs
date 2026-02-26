use clap::Parser;

use stellar_cli::commands::contract::invoke;
use stellar_registry_build::named_registry::PrefixedName;
use stellar_strkey::Contract;

use crate::commands::global;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of deployed contract. Can use prefix if not using verified registry.
    /// E.g. `unverified/<name>`
    pub contract: PrefixedName,

    /// Optional custom local name for the alias. If not provided, uses the name from the registry.
    pub local_name: Option<String>,

    /// Force overwrite if an alias with the same name already exists.
    #[arg(short, long)]
    pub force: bool,

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
    Registry(#[from] stellar_registry_build::Error),
    #[error(
        "Existing alias \"{1}\" exists. Overwrite with -f or provide a different local name like: \"create-alias {0} other-{1}\"."
    )]
    AliasExists(PrefixedName, String),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        let network_passphrase = self.config.get_network()?.network_passphrase;
        let alias = self.local_name.as_deref().unwrap_or(&self.contract.name);
        let contract = self.get_contract_id().await?;

        // Check if alias already exists
        if !self.force
            && self
                .config
                .locator
                .get_contract_id(alias, &network_passphrase)?
                .is_some()
        {
            return Err(Error::AliasExists(self.contract.clone(), alias.to_string()));
        }

        // Only create alias mapping, don't fetch wasm here
        self.config
            .locator
            .save_contract_id(&network_passphrase, &contract, alias)?;
        eprintln!("✅ Successfully registered contract alias '{alias}' for {contract}");
        Ok(())
    }

    pub async fn get_contract_id(&self) -> Result<Contract, Error> {
        let registry = &self.contract.registry(&self.config).await?;
        eprintln!("Fetching contract ID via registry...");
        Ok(registry.fetch_contract_id(&self.contract.name).await?)
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {

    use stellar_scaffold_test::{AssertExt, RegistryTest};

    fn publish_and_deploy(registry: &RegistryTest, name: &str) {
        // Path to the hello world contract WASM
        let wasm_path = registry.hello_wasm_v1();

        // First publish the contract
        registry
            .registry_cli("publish")
            .arg("--wasm")
            .arg(&wasm_path)
            .arg("--binver")
            .arg("0.0.2")
            .arg("--wasm-name")
            .arg(name)
            .assert()
            .success();

        // Then deploy it
        registry
            .registry_cli("deploy")
            .arg("--contract-name")
            .arg(name)
            .arg("--wasm-name")
            .arg(name)
            .arg("--version")
            .arg("0.0.2")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();
    }

    #[tokio::test]
    async fn test_run() {
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        publish_and_deploy(&registry, "hello");

        // Create test command for create-alias
        let cmd = registry.parse_cmd::<super::Cmd>(&["hello"]).unwrap();

        // Run the create-alias command
        cmd.run().await.unwrap();
        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/hello.json")
                .exists()
        );
    }

    #[tokio::test]
    async fn name_collision() {
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        publish_and_deploy(&registry, "hello");
        publish_and_deploy(&registry, "unverified/hello");

        // Run create-alias command
        registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .run()
            .await
            .unwrap();

        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/hello.json")
                .exists()
        );

        let contract_id = test_env
            .stellar("contract")
            .args(["alias", "show", "hello"])
            .assert()
            .stdout_as_str();

        // Run the create-alias command
        let cmd = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello"])
            .unwrap()
            .run()
            .await;

        // assert that cmd returns error (panics if result is ok)
        cmd.unwrap_err();

        // assert the alias still points at the same contract id
        assert_eq!(
            contract_id,
            test_env
                .stellar("contract")
                .args(["alias", "show", "hello"])
                .assert()
                .success()
                .stdout_as_str()
        );
    }

    #[tokio::test]
    async fn name_collision_with_overwrite() {
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        publish_and_deploy(&registry, "hello");
        publish_and_deploy(&registry, "unverified/hello");

        // Run create-alias command
        registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .run()
            .await
            .unwrap();

        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/hello.json")
                .exists()
        );

        let contract_id = test_env
            .stellar("contract")
            .args(["alias", "show", "hello"])
            .assert()
            .stdout_as_str();

        // Run the create-alias command
        let cmd = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello", "-f"])
            .unwrap()
            .run()
            .await;

        // assert that cmd succeeded
        cmd.unwrap();

        // assert the alias changed
        assert_ne!(
            contract_id,
            test_env
                .stellar("contract")
                .args(["alias", "show", "hello"])
                .assert()
                .success()
                .stdout_as_str()
        );
    }

    #[tokio::test]
    async fn alternate_local_name() {
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        publish_and_deploy(&registry, "hello");
        publish_and_deploy(&registry, "unverified/hello");

        // Run create-alias command
        registry
            .parse_cmd::<super::Cmd>(&["hello"])
            .unwrap()
            .run()
            .await
            .unwrap();

        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/hello.json")
                .exists()
        );

        let contract_id = test_env
            .stellar("contract")
            .args(["alias", "show", "hello"])
            .assert()
            .stdout_as_str();

        // Run the create-alias command
        let cmd = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello", "unverified_hello"])
            .unwrap()
            .run()
            .await;

        // assert that cmd succeeded
        cmd.unwrap();

        // assert the "hello" alias is the same
        assert_eq!(
            contract_id,
            test_env
                .stellar("contract")
                .args(["alias", "show", "hello"])
                .assert()
                .success()
                .stdout_as_str()
        );

        // assert we created a differently-named alias for unverified/hello
        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/unverified_hello.json")
                .exists()
        );
    }

    #[tokio::test]
    async fn unverified() {
        let registry = RegistryTest::new().await;
        let test_env = registry.clone().env;

        publish_and_deploy(&registry, "unverified/hello");

        // Create test command for install
        let cmd = registry
            .parse_cmd::<super::Cmd>(&["unverified/hello"])
            .unwrap();

        // Run the install command
        cmd.run().await.unwrap();
        assert!(
            test_env
                .cwd
                .join(".config/stellar/contract-ids/hello.json")
                .exists()
        );
    }
}
