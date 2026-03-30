#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use std::env;
use stellar_build::Network;
use syn::parse::{Parse, ParseStream, Result};
use syn::parse_macro_input;

mod asset;

pub(crate) fn manifest() -> std::path::PathBuf {
    std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("failed to find cargo manifest"))
        .join("Cargo.toml")
}

/// Generates a contract Client for a given contract.
/// The name should match a published contract or a contract in your current workspace.
///
/// # Usage
///
/// ```ignore
/// // For simple names (workspace contracts or registry names without hyphens):
/// import_contract_client!(registry);
///
/// // For hyphenated names or channel-prefixed registry paths:
/// import_contract_client!("unverified/guess-the-number");
/// ```
///
/// When using a string literal, the module name is derived from the contract
/// name with hyphens replaced by underscores (e.g., `guess_the_number`).
///
/// # Panics
///
/// This function may panic in the following situations:
/// - If `stellar_build::get_target_dir()` fails to retrieve the target directory
/// - If the input tokens cannot be parsed as a valid identifier or string literal
/// - If the directory path cannot be canonicalized
/// - If the canonical path cannot be converted to a string
#[proc_macro]
pub fn import_contract_client(wasm_binary: TokenStream) -> TokenStream {
    let WasmBinary { name, file } = parse_macro_input!(wasm_binary as WasmBinary);

    quote! {
        pub(crate) mod #name {
            #![allow(clippy::ref_option, clippy::too_many_arguments)]
            use super::soroban_sdk;
            soroban_sdk::contractimport!(file = #file);
        }
    }
    .into()
}

struct WasmBinary {
    pub name: Ident,
    pub file: String,
}

impl Parse for WasmBinary {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::LitStr) {
            let lit: syn::LitStr = input.parse()?;
            let raw = lit.value();
            let span = lit.span();

            let contract_name = raw.rsplit('/').next().unwrap_or(&raw);
            let mod_name = contract_name.replace('-', "_");
            let name = format_ident!("{}", mod_name, span = span);

            let wasm_path = resolve_wasm_path(contract_name, &raw, span)?;
            let file = wasm_path.display().to_string();
            Ok(Self { name, file })
        } else if lookahead.peek(syn::Ident) {
            let ident: Ident = input.parse()?;
            let ident_str = ident.to_string();
            let span = ident.span();

            let wasm_path = resolve_wasm_path(&ident_str, &ident_str, span)?;
            let file = wasm_path.display().to_string();
            Ok(Self { name: ident, file })
        } else {
            Err(lookahead.error())
        }
    }
}

fn resolve_wasm_path(
    contract_name: &str,
    full_name: &str,
    span: proc_macro2::Span,
) -> Result<std::path::PathBuf> {
    let target_dir = stellar_build::get_target_dir(&manifest()).unwrap();
    let local_path = target_dir.join(contract_name).with_extension("wasm");

    // 1. Check local build target
    if local_path.exists() {
        return Ok(local_path.canonicalize().expect("canonicalize failed"));
    }

    // 2. If STELLAR_NO_REGISTRY set to 1, error
    if let Ok(v) = env::var("STELLAR_NO_REGISTRY")
        && &v == "1"
    {
        return Err(syn::Error::new(
            span,
            "No local wasm found and STELLAR_NO_REGISTRY=1 so not checking Registry. \
            Download manually with `stellar registry download [wasm_name]`",
        ));
    }

    // 3. if var absent or set to something else, try to download
    download_from_registry(full_name, &local_path, span)
}

fn download_from_registry(
    full_name: &str,
    local_path: &std::path::Path,
    span: proc_macro2::Span,
) -> Result<std::path::PathBuf> {
    // 1. create `target/stellar/[network]` directory, if not already present
    let parent = local_path.parent().expect("no parent");
    if !parent.exists() {
        std::fs::create_dir_all(parent).expect("creating parent directory failed");
    }

    // 2. download using `stellar registry download`
    let status = std::process::Command::new("stellar")
        .args([
            "registry",
            "download",
            full_name,
            "--out-file",
            &local_path.display().to_string(),
        ])
        .status()
        .expect("failed to execute `stellar registry download`");

    // 3. check status
    if status.success() && local_path.exists() {
        Ok(local_path.canonicalize().expect("canonicalize failed"))
    } else {
        Err(syn::Error::new(
            span,
            "Could not find a Wasm with this name in local `target` directory \
            or in Stellar Registry. You can: \
            \n\n1. check the name & network and try again (https://stellar.rgstry.xyz) \
            \n2. add this Wasm to your local `target` directory manually \
            (perhaps by compiling a contract) \
            \n3. run `stellar registry download [wasm_name]`. \
            \n\nSet STELLAR_NO_REGISTRY=1 to skip registry lookup.",
        ))
    }
}

/// Generates a contract Client for a given asset.
/// It is expected that the name of an asset, e.g. "native" or "USDC:G1...."
///
/// # Panics
///
#[proc_macro]
pub fn import_asset(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input_str = syn::parse_macro_input!(input as syn::LitStr);
    asset::parse_literal(&input_str, &Network::passphrase_from_env()).into()
}
