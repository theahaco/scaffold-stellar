#![allow(dead_code)]
use std::ffi::OsString;

use ed25519_dalek::SigningKey;

use stellar_cli::{
    commands::contract::arg_parsing,
    config,
    xdr::{
        HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Memo, MuxedAccount, Operation,
        OperationBody, Preconditions, ScSpecEntry, ScVal, SequenceNumber, Transaction,
        TransactionExt, Uint256, VecM,
    },
};

use super::Error;

pub fn find_args_and_signers(
    contract_id: &stellar_strkey::Contract,
    mut slop: Vec<OsString>,
    spec_entries: &[ScSpecEntry],
) -> Result<(ScVal, Vec<SigningKey>), Error> {
    if !spec_entries.iter().any(is_constructor_fn) {
        return Ok((ScVal::Void, vec![]));
    }
    slop.insert(0, "__constructor".to_string().into());
    let res = arg_parsing::build_constructor_parameters(
        contract_id,
        &slop,
        spec_entries,
        &config::Args::default(),
    );
    match res {
        Ok((_, _, host_function_params, signers)) => {
            if host_function_params.function_name.len() > 64 {
                return Err(Error::FunctionNameTooLong(
                    host_function_params.function_name.to_string(),
                ));
            }
            let args = ScVal::Vec(Some(host_function_params.args.into()));
            Ok((args, signers))
        }
        Err(arg_parsing::Error::HelpMessage(help)) => Err(Error::ConstructorHelpMessage(help)),
        Err(e) => Err(Error::CannotParseArg {
            arg: slop
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
            error: e,
        }),
    }
}

pub fn build_invoke_contract_tx(
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

fn is_constructor_fn(spec_entries: &ScSpecEntry) -> bool {
    matches!(
        spec_entries,
        ScSpecEntry::FunctionV0(func) if func.name.to_string() == "__constructor"
    )
}
