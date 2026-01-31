use std::ffi::OsString;

use clap::Parser;
use soroban_rpc as rpc;
pub use soroban_spec_tools::contract as contract_spec;
use stellar_cli::{
    assembled::simulate_and_assemble_transaction,
    commands::contract::invoke,
    config::{self, UnresolvedMuxedAccount},
    fee,
    utils::rpc::get_remote_wasm_from_hash,
    xdr::{self, InvokeContractArgs, Limits, ScSpecEntry, ScString, ScVal, Uint256, WriteXdr},
};
use stellar_registry_build::{named_registry::PrefixedName, registry::Registry};

use crate::commands::global;

use super::deploy::util;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of published wasm to deploy from. Can use prefix if not using verified registry.
    /// E.g. `unverified/<name>`
    #[arg(long)]
    pub wasm_name: PrefixedName,

    /// Arguments for constructor
    #[arg(last = true, id = "CONSTRUCTOR_ARGS")]
    pub slop: Vec<OsString>,

    /// Version of the wasm to deploy
    #[arg(long)]
    pub version: Option<String>,

    /// Optional salt for deterministic contract address (hex-encoded 32 bytes)
    #[arg(long)]
    pub salt: Option<String>,

    /// Deployer account
    #[arg(long)]
    pub deployer: Option<UnresolvedMuxedAccount>,

    #[command(flatten)]
    pub config: global::Args,

    #[command(flatten)]
    pub fee: fee::Args,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Deploy(#[from] super::deploy::Error),
    #[error(transparent)]
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Rpc(#[from] rpc::Error),
    #[error(transparent)]
    SpecTools(#[from] soroban_spec_tools::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
    #[error(transparent)]
    ConfigAddress(#[from] config::address::Error),
    #[error(transparent)]
    Xdr(#[from] xdr::Error),
    #[error("Cannot parse contract spec")]
    CannotParseContractSpec,
    #[error("Constructor help message: {0}")]
    ConstructorHelpMessage(String),
    #[error("{0}")]
    InvalidReturnValue(String),
    #[error(transparent)]
    Registry(#[from] stellar_registry_build::Error),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        match self.invoke().await {
            Ok(contract_id) => {
                println!("Contract deployed successfully to {contract_id}");
                Ok(())
            }
            Err(Error::ConstructorHelpMessage(help)) => {
                println!("Constructor help message:\n{help}");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn hash(&self, registry: &Registry) -> Result<xdr::Hash, Error> {
        let mut slop = vec!["fetch_hash", "--wasm_name", &self.wasm_name.name];
        let version = self.version.clone().map(|v| format!("\"{v}\""));
        if let Some(version) = version.as_deref() {
            slop.push("--version");
            slop.push(version);
        }
        let res = registry
            .as_contract()
            .invoke_with_result(&slop, None, true)
            .await?;
        let res = res.trim_matches('"');
        Ok(res.parse().unwrap())
    }

    pub async fn wasm(&self, registry: &Registry) -> Result<Vec<u8>, Error> {
        Ok(
            get_remote_wasm_from_hash(&self.config.rpc_client()?, &self.hash(registry).await?)
                .await?,
        )
    }

    pub async fn spec_entries(&self, registry: &Registry) -> Result<Vec<ScSpecEntry>, Error> {
        Ok(contract_spec::Spec::new(&self.wasm(registry).await?)
            .map_err(|_| Error::CannotParseContractSpec)?
            .spec)
    }

    async fn invoke(&self) -> Result<stellar_strkey::Contract, Error> {
        let registry = self.wasm_name.registry(&self.config).await?;
        let client = self.config.rpc_client()?;
        let key = self.config.key_pair()?;
        let config = &self.config;

        let contract_address = registry.as_contract().sc_address();
        let contract_id = &registry.as_contract().id();
        let spec_entries = self.spec_entries(&registry).await?;
        let (args, signers) =
            util::find_args_and_signers(contract_id, self.slop.clone(), &spec_entries).await?;

        let deployer = if let Some(deployer) = &self.deployer {
            deployer
                .resolve_muxed_account(&self.config.locator, None)
                .await?
        } else {
            xdr::MuxedAccount::Ed25519(Uint256(key.verifying_key().to_bytes()))
        };

        // Build salt argument
        let salt_arg = if let Some(salt) = &self.salt {
            let bytes: [u8; 32] = hex::decode(salt)
                .map_err(|_| Error::InvalidReturnValue("Invalid salt hex".to_string()))?
                .try_into()
                .map_err(|_| Error::InvalidReturnValue("Salt must be 32 bytes".to_string()))?;
            ScVal::Bytes(xdr::ScBytes(bytes.try_into().unwrap()))
        } else {
            ScVal::Void
        };

        let invoke_contract_args = InvokeContractArgs {
            contract_address: contract_address.clone(),
            function_name: "deploy_unnammed".try_into().unwrap(),
            args: [
                ScVal::String(ScString(self.wasm_name.name.clone().try_into().unwrap())),
                self.version.clone().map_or(ScVal::Void, |s| {
                    ScVal::String(ScString(s.try_into().unwrap()))
                }),
                salt_arg,
                args,
                ScVal::Address(xdr::ScAddress::Account(deployer.account_id())),
            ]
            .try_into()
            .unwrap(),
        };

        // Get the account sequence number
        let public_strkey =
            stellar_strkey::ed25519::PublicKey(key.verifying_key().to_bytes()).to_string();
        let account_details = client.get_account(&public_strkey).await?;
        let sequence: i64 = account_details.seq_num.into();
        let tx =
            util::build_invoke_contract_tx(invoke_contract_args, sequence + 1, self.fee.fee, &key)?;
        let assembled = simulate_and_assemble_transaction(&client, &tx, None).await?;
        let mut txn = assembled.transaction().clone();
        println!("{}", txn.to_xdr_base64(Limits::none())?);
        if self.fee.build_only {
            println!("{}", txn.to_xdr_base64(Limits::none())?);
            std::process::exit(1);
        }
        txn = config
            .sign_soroban_authorizations(&txn, &signers)
            .await?
            .unwrap_or(txn);
        let return_value = client
            .send_transaction_polling(&config.sign(txn, false).await?)
            .await?
            .return_value()?;
        match return_value {
            ScVal::Address(xdr::ScAddress::Contract(xdr::ContractId(hash))) => {
                Ok(stellar_strkey::Contract(hash.0))
            }
            _ => Err(Error::InvalidReturnValue(
                "{return_value:#?} is not a contract address".to_string(),
            )),
        }
    }
}

// #[cfg(feature = "integration-tests")]
#[cfg(test)]
mod tests {
    use stellar_scaffold_test::RegistryTest;

    #[tokio::test]
    async fn simple() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

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

        // Deploy unnamed
        registry
            .registry_cli("deploy-unnamed")
            .env("RUST_LOGS", "trace")
            .env("RUSTLOGS", "trace")
            .arg("--wasm-name")
            .arg("hello")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();
    }

    #[tokio::test]
    async fn with_version() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();
        let v2 = registry.hello_wasm_v2();

        // Publish v1
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

        // Publish v2
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

        // Deploy unnamed with specific version
        registry
            .registry_cli("deploy-unnamed")
            .arg("--wasm-name")
            .arg("hello")
            .arg("--version")
            .arg("0.0.1")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();
    }

    #[tokio::test]
    async fn unverified() {
        let registry = RegistryTest::new().await;
        let v1 = registry.hello_wasm_v1();

        // First publish the contract to unverified registry
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

        // Deploy unnamed from unverified registry
        registry
            .registry_cli("deploy-unnamed")
            .arg("--wasm-name")
            .arg("unverified/hello")
            .arg("--")
            .arg("--admin=alice")
            .assert()
            .success();
    }
}
