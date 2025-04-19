use std::{collections::HashMap, ffi::OsString, path::PathBuf};

use clap::{value_parser, Parser};
use ed25519_dalek::SigningKey;
use heck::ToKebabCase;

use loam_sdk::soroban_sdk::xdr::{
    self, AccountId, HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Memo, MuxedAccount,
    Operation, OperationBody, Preconditions, ScSpecEntry, ScSpecFunctionV0, ScSpecTypeDef,
    ScString, ScSymbol, ScVal, SequenceNumber, Transaction, TransactionExt, Uint256, VecM,
};
use soroban_cli::{
    commands::{self, config, contract::invoke},
    fee,
};

use soroban_rpc as rpc;
use soroban_spec_tools::Spec;

use crate::testnet::{self, contract_address, invoke_smartdeploy};

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
    SmartdeployBuild(#[from] smartdeploy_build::Error),
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
        let res =
            invoke_smartdeploy(&["fetch_hash", "--contract_name", &self.published_name]).await?;
        let res = res.trim_matches('"');
        Ok(res.parse().unwrap())
    }

    pub async fn wasm(&self) -> Result<Vec<u8>, Error> {
        Ok(testnet::client()?
            .get_remote_wasm_from_hash(self.hash().await?)
            .await?)
    }

    pub async fn spec_entries(&self) -> Result<Vec<ScSpecEntry>, Error> {
        soroban_spec::read::from_wasm(&self.wasm().await?)
            .map_err(|_| Error::CannotParseContractSpec)
    }

    async fn invoke(&self) -> Result<(), Error> {
        let client = testnet::client()?;
        let key = self.config.key_pair()?;

        // Get the account sequence number
        let public_strkey =
            stellar_strkey::ed25519::PublicKey(key.verifying_key().to_bytes()).to_string();

        let contract_address = contract_address();

        let args = if self.slop.is_empty() {
            ScVal::Void
        } else {
            let (function_symbol_arg, final_args) = build_host_function_parameters(
                &self.deployed_name,
                &self.spec_entries().await?,
                &self.slop,
            )?;
            ScVal::Vec(Some(
                vec![
                    ScVal::Symbol(function_symbol_arg),
                    ScVal::Vec(Some(final_args.try_into().unwrap())),
                ]
                .try_into()
                .unwrap(),
            ))
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
        let res = client
            .prepare_and_send_transaction(
                &tx,
                &key,
                &[],
                &testnet::network_passphrase(),
                None,
                None,
            )
            .await?;
        let return_value = res.return_value()?;
        println!("{return_value:#?}");
        Ok(())
    }
}

fn build_host_function_parameters(
    name: &str,
    spec_entries: &[ScSpecEntry],
    slop: &[OsString],
) -> Result<(ScSymbol, Vec<ScVal>), Error> {
    let spec = soroban_spec_tools::Spec(Some(spec_entries.to_vec()));
    let mut cmd = clap::Command::new(name.to_owned())
        .no_binary_name(true)
        .term_width(300)
        .max_term_width(300);

    for ScSpecFunctionV0 { name, .. } in spec.find_functions()? {
        cmd = cmd.subcommand(build_custom_cmd(&name.to_utf8_string_lossy(), &spec)?);
    }
    cmd.build();
    let mut matches_ = cmd.get_matches_from(slop);
    let (function, matches_) = &matches_.remove_subcommand().unwrap();

    let func = spec.find_function(function)?;
    // create parsed_args in same order as the inputs to func
    let parsed_args = func
        .inputs
        .iter()
        .map(|i| {
            let name = i.name.to_string();
            if let Some(mut val) = matches_.get_raw(&name) {
                let mut s = val.next().unwrap().to_string_lossy().to_string();
                if matches!(i.type_, ScSpecTypeDef::Address) {
                    let cmd = commands::keys::address::Cmd {
                        name: s.clone(),
                        hd_path: Some(0),
                        locator: config::locator::Args::default(),
                    };
                    if let Ok(address) = cmd.public_key() {
                        s = address.to_string();
                    }
                }
                spec.from_string(&s, &i.type_)
                    .map_err(|error| Error::CannotParseArg { arg: name, error })
            } else if matches!(i.type_, ScSpecTypeDef::Option(_)) {
                Ok(ScVal::Void)
            } else if let Some(arg_path) = matches_.get_one::<PathBuf>(&fmt_arg_file_name(&name)) {
                if matches!(i.type_, ScSpecTypeDef::Bytes | ScSpecTypeDef::BytesN(_)) {
                    Ok(ScVal::try_from(
                        &std::fs::read(arg_path)
                            .map_err(|_| Error::MissingFileArg(arg_path.clone()))?,
                    )
                    .unwrap())
                } else {
                    let file_contents = std::fs::read_to_string(arg_path)
                        .map_err(|_| Error::MissingFileArg(arg_path.clone()))?;
                    spec.from_string(&file_contents, &i.type_)
                        .map_err(|error| Error::CannotParseArg { arg: name, error })
                }
            } else {
                Err(Error::MissingArgument(name))
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let function_symbol_arg = function
        .try_into()
        .map_err(|_| Error::FunctionNameTooLong(function.clone()))?;

    Ok((function_symbol_arg, parsed_args))
}

fn build_custom_cmd(name: &str, spec: &Spec) -> Result<clap::Command, Error> {
    let func = spec
        .find_function(name)
        .map_err(|_| Error::FunctionNotFoundInContractSpec(name.to_string()))?;

    // Parse the function arguments
    let inputs_map = &func
        .inputs
        .iter()
        .map(|i| (i.name.to_string(), i.type_.clone()))
        .collect::<HashMap<String, ScSpecTypeDef>>();
    let name: &'static str = Box::leak(name.to_string().into_boxed_str());
    let mut cmd = clap::Command::new(name)
        .no_binary_name(true)
        .term_width(300)
        .max_term_width(300);
    let kebab_name = name.to_kebab_case();
    if kebab_name != name {
        cmd = cmd.alias(kebab_name);
    }
    let func = spec.find_function(name).unwrap();
    let doc: &'static str = Box::leak(func.doc.to_utf8_string_lossy().into_boxed_str());
    let long_doc: &'static str = Box::leak(arg_file_help(doc).into_boxed_str());

    cmd = cmd.about(Some(doc)).long_about(long_doc);
    for (name, type_) in inputs_map {
        let mut arg = clap::Arg::new(name);
        let file_arg_name = fmt_arg_file_name(name);
        let mut file_arg = clap::Arg::new(&file_arg_name);
        arg = arg
            .long(name)
            .alias(name.to_kebab_case())
            .num_args(1)
            .value_parser(clap::builder::NonEmptyStringValueParser::new())
            .long_help(spec.doc(name, type_).unwrap());

        file_arg = file_arg
            .long(&file_arg_name)
            .alias(file_arg_name.to_kebab_case())
            .num_args(1)
            .hide(true)
            .value_parser(value_parser!(PathBuf))
            .conflicts_with(name);

        if let Some(value_name) = spec.arg_value_name(type_, 0) {
            let value_name: &'static str = Box::leak(value_name.into_boxed_str());
            arg = arg.value_name(value_name);
        }

        // Set up special-case arg rules
        arg = match type_ {
            xdr::ScSpecTypeDef::Bool => arg
                .num_args(0..1)
                .default_missing_value("true")
                .default_value("false")
                .num_args(0..=1),
            xdr::ScSpecTypeDef::Option(_val) => arg.required(false),
            xdr::ScSpecTypeDef::I256
            | xdr::ScSpecTypeDef::I128
            | xdr::ScSpecTypeDef::I64
            | xdr::ScSpecTypeDef::I32 => arg.allow_hyphen_values(true),
            _ => arg,
        };

        cmd = cmd.arg(arg);
        cmd = cmd.arg(file_arg);
    }
    Ok(cmd)
}

fn fmt_arg_file_name(name: &str) -> String {
    format!("{name}-file-path")
}

fn arg_file_help(docs: &str) -> String {
    format!(
        r#"{docs}
Usage Notes:
Each arg has a corresponding --<arg_name>-file-path which is a path to a file containing the corresponding JSON argument.
Note: The only types which aren't JSON are Bytes and Bytes which are raw bytes"#
    )
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
