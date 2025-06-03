#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use soroban_rpc as rpc;
use std::{env, path::PathBuf};
use stellar_cli::xdr::{
    ContractDataDurability, LedgerKey, LedgerKeyContractData, ScAddress, ScVal,
};
use stellar_cli::{
    config,
    utils::rpc::get_remote_wasm_from_hash,
    xdr::{self, ReadXdr},
};
use stellar_registry_cli::contract::NetworkContract;

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

/// Imports a contract client from the Stellar Registry.
///
/// Downloads the contract WASM from the registry if not already cached,
/// and generates a client with a `new` function that returns a client
/// for the specific contract ID.
///
/// # Arguments
/// * `name` - The name of the contract in the registry
/// * `network` - Optional network (defaults to "testnet")
///
/// # Example
/// ```ignore
/// import_contract!(hello_world);
/// import_contract!(hello_world, network = "mainnet");
/// ```
#[proc_macro]
pub fn import_contract(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as RegistryImportArgs);

    // Create a runtime for async operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    match rt.block_on(import_from_registry_impl(input)) {
        Ok(output) => output,
        Err(err) => syn::Error::new(proc_macro2::Span::call_site(), err)
            .to_compile_error()
            .into(),
    }
}

struct RegistryImportArgs {
    name: syn::Ident,
    network: Option<String>,
}

impl syn::parse::Parse for RegistryImportArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;

        let mut network = None;

        while !input.is_empty() {
            input.parse::<syn::Token![,]>()?;
            let key: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;

            match key.to_string().as_str() {
                "network" => {
                    let value: syn::LitStr = input.parse()?;
                    network = Some(value.value());
                }
                _ => return Err(syn::Error::new(key.span(), "Unknown parameter")),
            }
        }

        Ok(RegistryImportArgs {
            name,
            network,
        })
    }
}

async fn import_from_registry_impl(args: RegistryImportArgs) -> Result<TokenStream, String> {
    let contract_name = args.name.to_string();
    let network = args.network.as_deref().unwrap_or("testnet");

    // Create config for the network
    let config = create_config_for_network(network);

    // Get contract ID from registry using the CLI
    let contract_id = fetch_contract_id_from_registry(&config, &contract_name).await?;

    // Get WASM hash and ensure it's cached
    let wasm_path = ensure_wasm_cached_from_contract(&config, &contract_id).await?;

    // Generate the client code
    let name = args.name;
    let contract_id_str = contract_id.to_string();
    let wasm_path_str = wasm_path.to_str().ok_or("Invalid path")?;

    Ok(quote! {
        pub(crate) mod #name {
            #![allow(clippy::ref_option, clippy::too_many_arguments)]
            use super::soroban_sdk;

            soroban_sdk::contractimport!(file = #wasm_path_str);

            impl Client {
                /// Creates a new client for the contract deployed at the registry
                pub fn new(env: &soroban_sdk::Env) -> Self {
                    let contract_id = soroban_sdk::Address::from_string(
                        &soroban_sdk::String::from_str(env, #contract_id_str)
                    );
                    Self::new(env, &contract_id)
                }
            }
        }
    }
    .into())
}

fn create_config_for_network(network: &str) -> config::Args {
    use stellar_cli::config::{locator, network::Args as NetworkArgs};

    config::Args {
        locator: locator::Args::default(),
        network: NetworkArgs {
            network: Some(network.to_string()),
            ..Default::default()
        },
        ..Default::default()
    }
}

async fn fetch_contract_id_from_registry(
    config: &config::Args,
    contract_name: &str,
) -> Result<stellar_strkey::Contract, String> {
    use stellar_registry_cli::contract::NetworkContract;

    // Build the invoke arguments - fetch_contract_id function with contract name
    let args = vec!["fetch_contract_id", "--contract-name", contract_name];

    // Use the registry CLI to invoke the contract
    let result = config
        .invoke_registry(&args, None, true)
        .await
        .map_err(|e| format!("Failed to invoke registry: {e}"))?;

    // Parse the result - it should be a contract ID string
    let contract_id = result.trim().trim_matches('"');
    contract_id
        .parse()
        .map_err(|e| format!("Failed to parse contract ID '{contract_id}': {e}"))
}

async fn ensure_wasm_cached_from_contract(
    config: &config::Args,
    contract_id: &stellar_strkey::Contract,
) -> Result<PathBuf, String> {
    let client = config
        .rpc_client()
        .map_err(|e| format!("Failed to create RPC client: {e}"))?;

    // Get contract instance to find WASM hash
    let contract_key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(xdr::Hash(contract_id.0)),
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
    });

    let entries = client
        .get_ledger_entries(&[contract_key])
        .await
        .map_err(|e| format!("Failed to get contract data: {e}"))?;

    let entries = entries.entries.ok_or("No entries found")?;
    if entries.is_empty() {
        return Err("Contract not found".to_string());
    }

    // Extract WASM hash from contract instance
    let entry_data = xdr::LedgerEntryData::from_xdr_base64(&entries[0].xdr, xdr::Limits::none())
        .map_err(|e| format!("Failed to parse ledger entry: {e}"))?;

    let wasm_hash = match entry_data {
        xdr::LedgerEntryData::ContractData(data) => match &data.val {
            ScVal::ContractInstance(instance) => match &instance.executable {
                xdr::ContractExecutable::Wasm(hash) => hash.clone(),
                xdr::ContractExecutable::StellarAsset => return Err("Contract is not using WASM executable".to_string()),
            },
            _ => return Err("Invalid contract instance data".to_string()),
        },
        _ => return Err("Unexpected ledger entry type".to_string()),
    };

    // Check cache and download if needed
    ensure_wasm_cached(&client, &wasm_hash).await
}

async fn ensure_wasm_cached(
    client: &rpc::Client,
    wasm_hash: &xdr::Hash,
) -> Result<PathBuf, String> {
    let wasm_dir = get_wasm_cache_dir()?;
    let wasm_filename = format!("{wasm_hash}.wasm");
    let wasm_path = wasm_dir.join(&wasm_filename);

    if !wasm_path.exists() {
        // Use stellar-cli's fetch functionality to download WASM
        let wasm_bytes = get_remote_wasm_from_hash(client, wasm_hash)
            .await
            .map_err(|e| format!("Failed to download WASM: {e}"))?;

        // Save to cache
        std::fs::write(&wasm_path, &wasm_bytes)
            .map_err(|e| format!("Failed to save WASM to cache: {e}"))?;
    }

    Ok(wasm_path)
}

fn get_wasm_cache_dir() -> Result<PathBuf, String> {
    let data_dir = stellar_cli::config::data::data_local_dir()
        .map_err(|e| format!("Failed to get data directory: {e}"))?;

    let wasm_dir = data_dir.join("wasm");
    std::fs::create_dir_all(&wasm_dir)
        .map_err(|e| format!("Failed to create wasm directory: {e}"))?;

    Ok(wasm_dir)
}
