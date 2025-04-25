#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use std::env;

use quote::quote;

pub(crate) fn manifest() -> std::path::PathBuf {
    std::path::PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("failed to finde cargo manifest"),
    )
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
pub fn import_contract(tokens: TokenStream) -> TokenStream {
    let cargo_file = manifest();
    let mut dir = stellar_build::get_target_dir(&cargo_file)
        .unwrap()
        .join(tokens.to_string());
    let name = syn::parse::<syn::Ident>(tokens).expect("The input must be a valid identifier");
    dir.set_extension("wasm");
    let binding = dir.canonicalize().unwrap();
    let file = binding.to_str().unwrap();
    quote! {
        pub(crate) mod #name {
            #![allow(clippy::ref_option, clippy::too_many_arguments)]
            use soroban_sdk;
            soroban_sdk::contractimport!(file = #file);
        }
    }
    .into()
}
