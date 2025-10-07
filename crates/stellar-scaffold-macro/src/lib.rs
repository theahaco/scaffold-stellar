#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use stellar_xdr::curr as xdr;
use std::env;

use quote::quote;

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
    let mut dir = stellar_build::get_target_dir(&cargo_file)
        .unwrap()
        .join(tokens.to_string());
    let name = syn::parse::<syn::Ident>(tokens).expect("The input must be a valid identifier");
    dir.set_extension("wasm");
    let binding = dir.canonicalize().unwrap();
    let file = binding.to_str().unwrap();
    assert!(
        std::path::PathBuf::from(file).exists(),
        "The file does not exist: {file}"
    );
    quote! {
        pub(crate) mod #name {
            #![allow(clippy::ref_option, clippy::too_many_arguments)]
            use super::soroban_sdk;
            soroban_sdk::contractimport!(file = #file);
        }
    }
    .into()
}

fn parse_asset(str: &str) -> Result<xdr::Asset, xdr::Error> {
    if str == "native" {
        return Ok(xdr::Asset::Native);
    }
    let split: Vec<&str> = str.splitn(2, ':').collect();
    assert!(split.len() == 2, "invalid asset \"{str}\"");
    let code = split[0];
    let issuer: xdr::AccountId = split[1].parse()?;
    let re = regex::Regex::new("^[[:alnum:]]{1,12}$").expect("regex failed");
    assert!(re.is_match(code), "invalid asset \"{str}\"");
    let asset_code: xdr::AssetCode = code.parse()?;
    Ok(match asset_code {
        xdr::AssetCode::CreditAlphanum4(asset_code) => {
            xdr::Asset::CreditAlphanum4(xdr::AlphaNum4 { asset_code, issuer })
        }
        xdr::AssetCode::CreditAlphanum12(asset_code) => {
            xdr::Asset::CreditAlphanum12(xdr::AlphaNum12 { asset_code, issuer })
        }
    })
}

fn generate_asset_id(
    asset: &str,
) -> Result<stellar_strkey::Contract, xdr::Error> {
    use sha2::{Digest, Sha256};
    use xdr::WriteXdr;
    let asset = parse_asset(asset).unwrap();
    let network_passphrase = std::env::var("STELLAR_NETWORK_PASSPHRASE").unwrap_or_else(|_| "Standalone Network ; February 2017".to_owned());
    let network_id = xdr::Hash(Sha256::digest(network_passphrase.as_bytes()).into());
    let preimage = xdr::HashIdPreimage::ContractId(xdr::HashIdPreimageContractId {
        network_id,
        contract_id_preimage: xdr::ContractIdPreimage::Asset(asset.clone()),
    });
    let preimage_xdr = preimage.to_xdr(xdr::Limits::none())?;
    Ok(stellar_strkey::Contract(
        Sha256::digest(preimage_xdr).into(),
    ))
}

/// Generate the code to read the STELLAR_NETWORK environment variable
/// and call the generate_asset_id function
fn parse_asset_literal(lit_str: &syn::LitStr) -> TokenStream {
    let asset_code = lit_str.value();
    let asset_id = generate_asset_id(&asset_code).unwrap();
    let asset_id_str = stellar_strkey::Contract(asset_id.0).to_string();
    quote! {
        pub(crate) mod #lit_str {
            use soroban_sdk::Address;
            let env = soroban_sdk::env();
            let asset_address = Address::from_str(&env, #asset_id_str);
            soroban_sdk::token::StellarAssetClient::new(&env, &asset_address)
        }
    }
    .into()
}


/// Generates a contract Client for a given asset.
/// It is expected that the name of an asset, e.g. "native" or "USDC:G1...."
///
/// # Panics
///
#[proc_macro]
pub fn stellar_asset(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input_str = syn::parse_macro_input!(input as syn::LitStr);
    let asset = parse_asset_literal(&input_str);

    // Return the generated code as a TokenStream
    asset
}
