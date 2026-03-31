#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::env;
use stellar_build::Network;
use syn::parse::{Parse, ParseStream, Result};
use syn::{Ident, LitStr, parse_macro_input};

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
    let WasmBinary { mod_name, file } = parse_macro_input!(wasm_binary as WasmBinary);

    quote! {
        pub(crate) mod #mod_name {
            #![allow(clippy::ref_option, clippy::too_many_arguments)]
            use super::soroban_sdk;
            soroban_sdk::contractimport!(file = #file);
        }
    }
    .into()
}

struct WasmBinary {
    pub mod_name: Ident,
    pub file: String,
}

impl Parse for WasmBinary {
    fn parse(input: ParseStream) -> Result<Self> {
        let (lookup_name, mod_name) = parse_contract_name(input)?;
        let wasm_path = resolve_wasm_path(&lookup_name, &mod_name)?;
        let file = wasm_path.display().to_string();
        Ok(Self { mod_name, file })
    }
}

/// Parse a contract name (either an identifier like `registry` or a string like
/// `"unverified/guess-the-number"`) and return the raw lookup name along with a
/// sanitized module-level identifier.
fn parse_contract_name(input: ParseStream) -> Result<(String, Ident)> {
    let span = input.span();
    let lookup_name = if input.peek(LitStr) {
        input.parse::<LitStr>()?.value()
    } else {
        input.parse::<Ident>()?.to_string()
    };

    if regex::Regex::new(r"(^/)|(/$)")
        .unwrap()
        .is_match(&lookup_name)
    {
        return Err(syn::Error::new(
            span,
            format!("bad leading/trailing slash: `{lookup_name}`"),
        ));
    }

    // Derive a valid Rust identifier for the module name
    // e.g., "unverified/guess-the-number" -> "guess_the_number"
    let mod_name_str = lookup_name
        .rsplit('/')
        .next()
        .unwrap_or(&lookup_name)
        .replace('-', "_");
    let mod_name = Ident::new(&mod_name_str, span);

    Ok((lookup_name, mod_name))
}

fn resolve_wasm_path(lookup_name: &str, mod_name: &Ident) -> Result<std::path::PathBuf> {
    let target_dir = stellar_build::get_target_dir(&manifest()).unwrap();
    let local_path = target_dir.join(mod_name.to_string()).with_extension("wasm");

    // 1. Check local build target
    if local_path.exists() {
        return Ok(local_path.canonicalize().expect("canonicalize failed"));
    }

    // 2. If STELLAR_NO_REGISTRY set to 1, error
    if let Ok(v) = env::var("STELLAR_NO_REGISTRY")
        && &v == "1"
    {
        return Err(syn::Error::new(
            mod_name.span(),
            format!(
                "No local wasm found and STELLAR_NO_REGISTRY=1 so not checking Registry. \
                Download manually with `stellar registry download {lookup_name}`"
            ),
        ));
    }

    // 3. if var absent or set to something else, try to download
    download_from_registry(lookup_name, &local_path, mod_name.span())
}

fn download_from_registry(
    lookup_name: &str,
    local_path: &std::path::Path,
    span: Span,
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
            lookup_name,
            "--out-file",
            &local_path.display().to_string(),
        ])
        .status()
        .expect(
            "failed to execute `stellar registry download`; try `cargo install stellar-registry-cli` and try again",
        );

    // 3. check status
    if status.success() && local_path.exists() {
        Ok(local_path.canonicalize().expect("canonicalize failed"))
    } else {
        let local_path = local_path.display().to_string();
        Err(syn::Error::new(
            span,
            format!(
                "Could not find Wasm `{lookup_name}`. Checked: \
                \n\n• {local_path} \
                \n• `stellar registry download {lookup_name}` \
                \n\nYou can: \
                \n\n1. check the name & network and try again (https://stellar.rgstry.xyz) \
                \n2. add this Wasm to your local `target` directory manually \
                (perhaps by compiling a contract) \
                \n3. run `stellar registry download {lookup_name}` yourself. \
                \n\nSet STELLAR_NO_REGISTRY=1 to skip registry lookup."
            ),
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

#[cfg(test)]
mod test {
    use super::*;
    use syn::parse::Parser;

    #[test]
    fn parse_simple_name() {
        let (lookup_name, mod_name) = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!(registry))
            .unwrap();
        assert_eq!(mod_name.to_string(), "registry");
        assert_eq!(lookup_name, "registry");
    }

    #[test]
    fn parse_channel_hyphenated_name() {
        let (lookup_name, mod_name) = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("guess-the-number"))
            .unwrap();
        assert_eq!(mod_name.to_string(), "guess_the_number");
        assert_eq!(lookup_name, "guess-the-number");
    }

    #[test]
    fn parse_channel_prefixed_name() {
        let (lookup_name, mod_name) = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("unverified/guess-the-number"))
            .unwrap();
        assert_eq!(mod_name.to_string(), "guess_the_number");
        assert_eq!(lookup_name, "unverified/guess-the-number");
    }

    #[test]
    fn parse_channel_simple_name() {
        let (lookup_name, mod_name) = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("unverified/hello"))
            .unwrap();
        assert_eq!(mod_name.to_string(), "hello");
        assert_eq!(lookup_name, "unverified/hello");
    }

    #[test]
    fn parse_underscored_name() {
        let (lookup_name, mod_name) = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("my_contract"))
            .unwrap();
        assert_eq!(mod_name, "my_contract");
        assert_eq!(lookup_name, "my_contract");
    }

    #[test]
    fn error_trailing_slash() {
        let err = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("unverified/"))
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("bad leading/trailing slash: `unverified/`"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn error_leading_slash() {
        let err = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("/guess-the-number"))
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("bad leading/trailing slash: `/guess-the-number`"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn multiple_slashes_returns_final_as_mod_name() {
        let (lookup_name, mod_name) = (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("a/b/c"))
            .unwrap();
        assert_eq!(mod_name, "c");
        assert_eq!(lookup_name, "a/b/c");
    }

    #[test]
    #[should_panic(expected = "Ident is not allowed to be empty")]
    fn error_empty_string() {
        (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!(""))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "not a valid Ident")]
    fn error_starts_with_digit() {
        (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("123bad"))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "not a valid Ident")]
    fn error_invalid_characters() {
        (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("hello world"))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "not a valid Ident")]
    fn error_channel_prefixed_starts_with_digit() {
        (|input: ParseStream| parse_contract_name(input))
            .parse2(quote!("unverified/1bad"))
            .unwrap();
    }
}
