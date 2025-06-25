use clap::value_parser;

use heck::ToKebabCase;
use soroban_spec_tools::Spec;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use stellar_cli::{
    config::{self, sc_address},
    xdr::{self, ScSpecTypeDef},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("function {0} was not found in the contract")]
    FunctionNotFoundInContractSpec(String),
    #[error(transparent)]
    Xdr(#[from] xdr::Error),
    #[error(transparent)]
    StrVal(#[from] soroban_spec_tools::Error),
    #[error(transparent)]
    ScAddress(#[from] sc_address::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
    // #[error("")]
    // HelpMessage(String),
}

pub fn build_custom_cmd(name: &str, spec: &Spec) -> Result<clap::Command, Error> {
    let func = spec
        .find_function(name)
        .map_err(|_| Error::FunctionNotFoundInContractSpec(name.to_string()))?;

    // Parse the function arguments
    let inputs_map = &func
        .inputs
        .iter()
        .map(|i| (i.name.to_utf8_string().unwrap(), i.type_.clone()))
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
            .long_help(spec.doc(name, type_)?);

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
            ScSpecTypeDef::Bool => arg
                .num_args(0..1)
                .default_missing_value("true")
                .default_value("false")
                .num_args(0..=1),
            ScSpecTypeDef::Option(_val) => arg.required(false),
            ScSpecTypeDef::I256 | ScSpecTypeDef::I128 | ScSpecTypeDef::I64 | ScSpecTypeDef::I32 => {
                arg.allow_hyphen_values(true)
            }
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
        r"{docs}
Usage Notes:
Each arg has a corresponding --<arg_name>-file-path which is a path to a file containing the corresponding JSON argument.
Note: The only types which aren't JSON are Bytes and BytesN, which are raw bytes"
    )
}
