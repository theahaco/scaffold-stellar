#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use std::env;
use stellar_build::Network;

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
pub fn import_contract_client(tokens: TokenStream) -> TokenStream {
    let cargo_file = manifest();
    let name =
        syn::parse::<syn::Ident>(tokens.clone()).expect("The input must be a valid identifier");
    let name_str = name.to_string();

    let wasm_path = resolve_wasm_path(&cargo_file, &name_str);
    let file = wasm_path.to_str().unwrap();

    quote! {
        pub(crate) mod #name {
            #![allow(clippy::ref_option, clippy::too_many_arguments)]
            use super::soroban_sdk;
            soroban_sdk::contractimport!(file = #file);
        }
    }
    .into()
}

fn resolve_wasm_path(cargo_file: &std::path::Path, name: &str) -> std::path::PathBuf {
    let target_dir = stellar_build::get_target_dir(cargo_file).unwrap();
    let mut local_path = target_dir.join(name);
    local_path.set_extension("wasm");

    // 1. Check local build target
    if local_path.exists() {
        return local_path.canonicalize().unwrap();
    }

    // 2. Try registry download (unless opted out)
    if env::var("STELLAR_NO_REGISTRY").is_err()
        && let Ok(path) = download_from_registry(name, &target_dir)
    {
        return path;
    }

    panic!(
        "Could not find wasm for '{name}'. Checked local target ({}) and registry. \
         Build the contract or ensure registry access. \
         Set STELLAR_NO_REGISTRY=1 to skip registry lookup.",
        local_path.display()
    );
}

fn download_from_registry(
    name: &str,
    target_dir: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let output = target_dir.join(name).with_extension("wasm");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let output_str = output.display().to_string();
    let status = std::process::Command::new("stellar")
        .args(["registry", "download", name, "--out-file", &output_str])
        .status()?;
    if status.success() && output.exists() {
        Ok(output.canonicalize()?)
    } else {
        Err("registry download failed".into())
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
