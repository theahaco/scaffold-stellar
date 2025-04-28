use std::{ffi::OsString, path::PathBuf};

use clap::Parser;
use ed25519_dalek::SigningKey;

use loam_sdk::soroban_sdk::xdr::{
    self, AccountId, HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Memo, MuxedAccount,
    Operation, OperationBody, Preconditions, ScSpecEntry, ScString, ScVal, SequenceNumber,
    Transaction, TransactionExt, Uint256, VecM,
};
use stellar_cli::{
    assembled::simulate_and_assemble_transaction, commands::contract::invoke, config, fee,
    utils::rpc::get_remote_wasm_from_hash,
};

use soroban_rpc as rpc;
pub use soroban_spec_tools::contract as contract_spec;

use crate::testnet::{self, contract_address, invoke_registry};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Name of contract to be deployed
    #[arg(long, visible_alias = "deploy-as")]
    pub deployed_name: String,
    /// Name of published contract to deploy from
    #[arg(long)]
    pub published_name: String,
    /// Function name as subcommand, then arguments for that function as `--arg-name value`
    #[arg(last = true, id = "CONTRACT_FN_AND_ARGS")]
    pub slop: Vec<OsString>,

    #[command(flatten)]
    pub config: config::Args,
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
    StellarBuild(#[from] stellar_build::Error),
    #[error(transparent)]
    Install(#[from] super::install::Error),
    #[error(transparent)]
    Rpc(#[from] rpc::Error),
    #[error(transparent)]
    SpecTools(#[from] soroban_spec_tools::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
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
        error: soroban_spec_tools::Error,
    },
    #[error("function name {0} is too long")]
    FunctionNameTooLong(String),
    #[error("Missing file argument {0:#?}")]
    MissingFileArg(PathBuf),
    #[error("Missing argument {0}")]
    MissingArgument(String),
}

impl Cmd {
    pub async fn run(&self) -> Result<(), Error> {
        self.invoke().await?;
        Ok(())
    }

    pub async fn hash(&self) -> Result<xdr::Hash, Error> {
        let res = invoke_registry(&["fetch_hash", "--contract_name", &self.published_name]).await?;
        let res = res.trim_matches('"');
        Ok(res.parse().unwrap())
    }

    pub async fn wasm(&self) -> Result<Vec<u8>, Error> {
        Ok(get_remote_wasm_from_hash(&testnet::client()?, &self.hash().await?).await?)
    }

    pub async fn spec_entries(&self) -> Result<Vec<ScSpecEntry>, Error> {
        Ok(contract_spec::Spec::new(&self.wasm().await?)
            .map_err(|_| Error::CannotParseContractSpec)?
            .spec)
    }

    async fn invoke(&self) -> Result<(), Error> {
        let client = testnet::client()?;
        let key = self.config.key_pair()?;
        let config = &self.config;

        // Get the account sequence number
        let public_strkey =
            stellar_strkey::ed25519::PublicKey(key.verifying_key().to_bytes()).to_string();

        let contract_address = contract_address();
        let contract_id = &testnet::contract_id_strkey();
        let spec_entries = self.spec_entries().await?;

        let (args, signers) = if self.slop.is_empty() {
            (ScVal::Void, vec![])
        } else {
            let (_, _, host_function_params, signers) =
                stellar_cli::commands::contract::arg_parsing::build_host_function_parameters(
                    contract_id,
                    &self.slop,
                    &spec_entries,
                    &config::Args::default(),
                )
                .unwrap();

            (
                ScVal::Vec(Some(
                    vec![
                        ScVal::Symbol(host_function_params.function_name),
                        ScVal::Vec(Some(host_function_params.args.into())),
                    ]
                    .try_into()
                    .unwrap(),
                )),
                signers,
            )
        };

        let invoke_contract_args = InvokeContractArgs {
            contract_address: contract_address.clone(),
            function_name: "deploy".try_into().unwrap(),
            args: [
                ScVal::String(ScString(self.published_name.clone().try_into().unwrap())),
                ScVal::Void,
                ScVal::String(ScString(self.deployed_name.clone().try_into().unwrap())),
                ScVal::Address(xdr::ScAddress::Account(AccountId(
                    xdr::PublicKey::PublicKeyTypeEd25519(Uint256(key.verifying_key().to_bytes())),
                ))),
                ScVal::Void,
                args,
            ]
            .try_into()
            .unwrap(),
        };

        let account_details = client.get_account(&public_strkey).await?;
        let sequence: i64 = account_details.seq_num.into();
        let tx = build_invoke_contract_tx(invoke_contract_args, sequence + 1, self.fee.fee, &key)?;
        let assembled = simulate_and_assemble_transaction(&client, &tx).await?;
        let mut txn = assembled.transaction().clone();
        txn = config
            .sign_soroban_authorizations(&txn, &signers)
            .await?
            .unwrap_or(txn);
        let res = client
            .send_transaction_polling(&config.sign_with_local_key(txn).await?)
            .await?;

        let return_value = res.return_value()?;
        println!("{return_value:#?}");
        Ok(())
    }
}

fn build_invoke_contract_tx(
    parameters: InvokeContractArgs,
    sequence: i64,
    fee: u32,
    key: &SigningKey,
) -> Result<Transaction, Error> {
    let op = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function: HostFunction::InvokeContract(parameters),
            auth: VecM::default(),
        }),
    };
    Ok(Transaction {
        source_account: MuxedAccount::Ed25519(Uint256(key.verifying_key().to_bytes())),
        fee,
        seq_num: SequenceNumber(sequence),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![op].try_into()?,
        ext: TransactionExt::V0,
    })
}
