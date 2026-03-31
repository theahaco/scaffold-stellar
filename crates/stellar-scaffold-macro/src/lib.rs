#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
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
/// It is expected that the name should be the same as the published contract or a contract in your current workspace.
///
/// # Panics
///
/// This function may panic in the following situations:
/// - If `stellar_build::get_target_dir()` fails to retrieve the target directory
/// - If the input tokens cannot be parsed as a valid identifier
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
        let name = input.parse()?;
        let wasm_path = resolve_wasm_path(&name)?;
        let file = wasm_path.display().to_string();
        Ok(Self { name, file })
    }
}

fn resolve_wasm_path(name: &Ident) -> Result<std::path::PathBuf> {
    let target_dir = stellar_build::get_target_dir(&manifest()).unwrap();
    let local_path = target_dir.join(name.to_string()).with_extension("wasm");

    // 1. Check local build target
    if local_path.exists() {
        return Ok(local_path.canonicalize().expect("canonicalize failed"));
    }

    // 2. If STELLAR_NO_REGISTRY set to 1, error
    if let Ok(v) = env::var("STELLAR_NO_REGISTRY")
        && &v == "1"
    {
        return Err(syn::Error::new(
            name.span(),
            "No local wasm found and STELLAR_NO_REGISTRY=1 so not checking Registry. \
            Download manually with `stellar registry download [wasm_name]`",
        ));
    }

    // 3. if var absent or set to something else, try to download
    download_from_registry(name, &local_path)
}

fn download_from_registry(
    name: &Ident,
    local_path: &std::path::Path,
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
            &name.to_string(),
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
            name.span(),
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
/// It produces 2 modules that can be used either in your contract code or in unit tests.
/// As the first argument, it expects the name of an asset, e.g. "native" or "USDC:G1...."
/// To generate the contract id for the asset, it uses the `STELLAR_NETWORK` environment variable,
/// which could be either `local`, `testnet`, `futurenet` or `mainnet`. (uses `local` if not set)
///
/// Example:
/// ```ignore
/// import_asset!("native");
/// ```
/// Can be used in unit tests as follows:
/// ```ignore
/// let env = &Env::default();
/// let admin = &Address::generate(env);
/// let sac = test_native::register(env, admin);
/// let client = test_native::stellar_asset_client(env, &sac);
/// assert_eq!(client.admin(), *admin);
/// assert_eq!(client.balance(admin), 1000000000);
/// ```
/// And in your contract code:
/// ```ignore
/// ```
/// # Panics
///
#[proc_macro]
pub fn import_asset(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input_str = syn::parse_macro_input!(input as syn::LitStr);
    asset::parse_literal(&input_str).into()
}
