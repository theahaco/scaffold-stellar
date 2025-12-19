#![allow(dead_code)]
use std::{ffi::OsString, path::PathBuf};

use clap::Parser;
use soroban_rpc as rpc;
pub use soroban_spec_tools::contract as contract_spec;
use stellar_cli::{
    assembled::simulate_and_assemble_transaction,
    commands::contract::invoke,
    config::{self, UnresolvedMuxedAccount},
    fee,
    utils::rpc::get_remote_wasm_from_hash,
    xdr::{
        self, AccountId, InvokeContractArgs, Limits, ScSpecEntry, ScString, ScVal, Uint256,
        WriteXdr,
    },
};
use stellar_registry_build::{named_registry::PrefixedName, registry::Registry};

use crate::commands::global;

mod util;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of contract to be deployed
    #[arg(long, visible_alias = "deploy-as")]
    pub contract_name: PrefixedName,
    /// Name of published contract to deploy from
    #[arg(long)]
    pub wasm_name: String,
    /// Arguments for constructor
    #[arg(last = true, id = "CONSTRUCTOR_ARGS")]
    pub slop: Vec<OsString>,
    /// Version of the wasm to deploy
    #[arg(long)]
    pub version: Option<String>,
    /// Optional deployer, by default is registry contract itself
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
    Invoke(#[from] invoke::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Install(#[from] super::create_alias::Error),
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
    #[error("argument count ({current}) surpasses maximum allowed count ({maximum})")]
    MaxNumberOfArgumentsReached { current: usize, maximum: usize },
    #[error("function {0} was not found in the contract")]
    FunctionNotFoundInContractSpec(String),
    #[error("parsing argument {arg}: {error}")]
    CannotParseArg {
        arg: String,
        error: stellar_cli::commands::contract::arg_parsing::Error,
    },
    #[error("function name {0} is too long")]
    FunctionNameTooLong(String),
    #[error("Missing file argument {0:#?}")]
    MissingFileArg(PathBuf),
    #[error("Missing argument {0}")]
    MissingArgument(String),
    #[error("Constructor help message: {0}")]
    ConstructorHelpMessage(String),
    #[error("{0}")]
    InvalidReturnValue(String),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        match self.invoke().await {
            Ok(contract_id) => {
                println!(
                    "Contract {} deployed successfully to {contract_id}",
                    self.contract_name.name
                );
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
        let res = registry
            .as_contract()
            .invoke_with_result(&["fetch_hash", "--wasm_name", &self.wasm_name], None, true)
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
        let registry = self.contract_name.registry(&self.config).await?;
        let client = self.config.rpc_client()?;
        let key = self.config.key_pair()?;
        let config = &self.config;

        let contract_address = registry.as_contract().sc_address();
        let contract_id = &registry.as_contract().id();
        let spec_entries = self.spec_entries(&registry).await?;
        let (args, signers) =
            util::find_args_and_signers(contract_id, self.slop.clone(), &spec_entries).await?;
        let deployer = if let Some(deployer) = &self.deployer {
            Some(
                deployer
                    .resolve_muxed_account(&self.config.locator, None)
                    .await?,
            )
        } else {
            None
        };
        let invoke_contract_args = InvokeContractArgs {
            contract_address: contract_address.clone(),
            function_name: "deploy".try_into().unwrap(),
            args: [
                ScVal::String(ScString(self.wasm_name.clone().try_into().unwrap())),
                self.version.clone().map_or(ScVal::Void, |s| {
                    ScVal::String(ScString(s.try_into().unwrap()))
                }),
                ScVal::String(ScString(
                    self.contract_name.name.clone().try_into().unwrap(),
                )),
                ScVal::Address(xdr::ScAddress::Account(AccountId(
                    xdr::PublicKey::PublicKeyTypeEd25519(Uint256(key.verifying_key().to_bytes())),
                ))),
                args,
                deployer.map_or(ScVal::Void, |muxed_account| {
                    ScVal::Address(xdr::ScAddress::Account(muxed_account.account_id()))
                }),
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
