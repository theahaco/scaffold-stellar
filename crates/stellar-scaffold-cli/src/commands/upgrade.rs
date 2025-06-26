use crate::commands::build::env_toml::{Account, Contract, Environment, Network};
use clap::Parser;
use degit::degit;
use dialoguer::{Confirm, Input, Select};
use indexmap::IndexMap;
use std::fs;
use std::fs::{create_dir_all, metadata, read_dir, write};
use std::io;
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut, Item, Table};

use crate::{arg_parsing, commands::build};
use stellar_cli::print::Print;

const FRONTEND_TEMPLATE: &str = "https://github.com/AhaLabs/scaffold-stellar-frontend";

/// A command to upgrade an existing Soroban workspace to a scaffold project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the existing workspace (defaults to current directory)
    #[arg(default_value = ".")]
    pub workspace_path: PathBuf,
}

/// Errors that can occur during upgrade
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to clone template: {0}")]
    DegitError(String),
    #[error(
        "Workspace path contains invalid UTF-8 characters and cannot be converted to a string"
    )]
    InvalidWorkspacePathEncoding,
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error("No Cargo.toml found in workspace path")]
    NoCargoToml,
    #[error("No contracts/ directory found in workspace path")]
    NoContractsDirectory,
    #[error("Invalid package name in Cargo.toml")]
    InvalidPackageName,
    #[error("Failed to parse TOML: {0}")]
    TomlParseError(#[from] toml_edit::TomlError),
    #[error("Failed to serialize TOML: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),
    #[error("Failed to deserialize TOML: {0}")]
    TomlDeserializeError(#[from] toml::de::Error),
    #[error(transparent)]
    BuildError(#[from] build::Error),
    #[error("Failed to get constructor arguments: {0}")]
    ConstructorArgsError(String),
    #[error("WASM file not found for contract '{0}'. Please build the contract first.")]
    WasmFileNotFound(String),
    #[error(transparent)]
    Clap(#[from] clap::Error),
    #[error(transparent)]
    SorobanSpecTools(#[from] soroban_spec_tools::contract::Error),
    #[error(transparent)]
    CopyError(#[from] fs_extra::error::Error),
}

impl Cmd {
    /// Run the upgrade command
    ///
    /// # Example:
    ///
    /// ```
    /// /// From the command line
    /// stellar scaffold upgrade /path/to/workspace
    /// ```
    pub async fn run(
        &self,
        global_args: &stellar_cli::commands::global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        printer.infoln(format!(
            "Upgrading Soroban workspace to scaffold project in {:?}",
            self.workspace_path
        ));

        // Validate workspace
        self.validate_workspace()?;

        // Create temporary directory for frontend template
        let temp_dir = tempfile::tempdir().map_err(Error::IoError)?;
        let temp_path = temp_dir.path();

        printer.infoln("Downloading frontend template...");
        Self::clone_frontend_template(temp_path)?;

        printer.infoln("Copying frontend files...");
        self.copy_frontend_files(temp_path)?;

        printer.infoln("Setting up environment file...");
        self.setup_env_file()?;

        printer.infoln("Creating environments.toml...");
        self.create_environments_toml().await?;

        printer.checkln(format!(
            "Workspace successfully upgraded to scaffold project at {:?}",
            self.workspace_path
        ));

        Ok(())
    }

    fn validate_workspace(&self) -> Result<(), Error> {
        // Check for Cargo.toml
        let cargo_toml = self.workspace_path.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(Error::NoCargoToml);
        }

        // Check for contracts/ directory
        let contracts_dir = self.workspace_path.join("contracts");
        if !contracts_dir.exists() {
            return Err(Error::NoContractsDirectory);
        }

        Ok(())
    }

    fn clone_frontend_template(temp_path: &Path) -> Result<(), Error> {
        let temp_str = temp_path
            .to_str()
            .ok_or(Error::InvalidWorkspacePathEncoding)?;

        degit(FRONTEND_TEMPLATE, temp_str);

        if metadata(temp_path).is_err() || read_dir(temp_path)?.next().is_none() {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {temp_str}: directory is empty or missing",
            )));
        }

        Ok(())
    }

    fn copy_frontend_files(&self, temp_path: &Path) -> Result<(), Error> {
        // Files and directories to skip (don't copy from template)
        let skip_items = ["contracts", "environments.toml", "Cargo.toml"];

        // Copy all items from template except the ones we want to skip
        for entry in read_dir(temp_path)? {
            let entry = entry?;
            let item_name = entry.file_name();

            // Skip items that shouldn't be copied
            if let Some(name_str) = item_name.to_str() {
                if skip_items.contains(&name_str) {
                    continue;
                }
            }

            let src = entry.path();
            let dest = self.workspace_path.join(&item_name);

            // Don't overwrite existing files/directories
            if dest.exists() {
                continue;
            }

            if src.is_dir() {
                let copy_options = fs_extra::dir::CopyOptions::new()
                    .overwrite(false) // Don't overwrite existing files
                    .skip_exist(true); // Skip files that already exist

                fs_extra::dir::copy(&src, &self.workspace_path, &copy_options)?;
            } else {
                let copy_options = fs_extra::file::CopyOptions::new().overwrite(false); // Don't overwrite existing files

                fs_extra::file::copy(&src, &dest, &copy_options)?;
            }
        }

        // Create packages directory if it doesn't exist
        let packages_dir = self.workspace_path.join("packages");
        if !packages_dir.exists() {
            create_dir_all(&packages_dir)?;
        }

        Ok(())
    }

    async fn create_environments_toml(&self) -> Result<(), Error> {
        let env_path = self.workspace_path.join("environments.toml");

        // Don't overwrite existing environments.toml
        if env_path.exists() {
            return Ok(());
        }

        // Discover contracts by looking in contracts/ directory
        let contracts = self.discover_contracts()?;

        // Build contracts to get WASM files for constructor arg analysis
        self.build_contracts().await?;

        // Get constructor args for each contract
        let contract_configs = contracts
            .iter()
            .map(|contract_name| {
                let constructor_args = self.get_constructor_args(contract_name)?;
                Ok((
                    contract_name.clone().into_boxed_str(),
                    Contract {
                        constructor_args,
                        ..Default::default()
                    },
                ))
            })
            .collect::<Result<IndexMap<_, _>, Error>>()?;

        let env_config = Environment {
            accounts: Some(vec![Account {
                name: "default".to_string(),
                default: true,
            }]),
            network: Network {
                name: None,
                rpc_url: Some("http://localhost:8000/rpc".to_string()),
                network_passphrase: Some("Standalone Network ; February 2017".to_string()),
                rpc_headers: None,
                run_locally: true,
            },
            contracts: (!contract_configs.is_empty()).then_some(contract_configs),
        };

        let mut doc = DocumentMut::new();

        // Add development environment
        let mut dev_table = Table::new();

        // Add accounts
        let mut accounts_array = toml_edit::Array::new();
        accounts_array.push("default");
        dev_table["accounts"] = Item::Value(accounts_array.into());

        // Add network
        let mut network_table = Table::new();
        network_table["rpc-url"] = value(env_config.network.rpc_url.as_ref().unwrap());
        network_table["network-passphrase"] =
            value(env_config.network.network_passphrase.as_ref().unwrap());
        network_table["run-locally"] = value(env_config.network.run_locally);
        dev_table["network"] = Item::Table(network_table);

        // Add contracts
        let contracts_table = env_config
            .contracts
            .as_ref()
            .map(|contracts| {
                contracts
                    .iter()
                    .map(|(name, config)| {
                        let mut contract_constructor_args = Table::new();
                        if let Some(args) = &config.constructor_args {
                            contract_constructor_args["constructor_args"] = value(args);
                        }
                        // Convert hyphens to underscores for contract names in TOML
                        let contract_key = name.replace('-', "_");
                        (contract_key, Item::Table(contract_constructor_args))
                    })
                    .collect::<Table>()
            })
            .unwrap_or_default();

        dev_table["contracts"] = Item::Table(contracts_table);

        doc["development"] = Item::Table(dev_table);

        write(&env_path, doc.to_string())?;

        Ok(())
    }

    fn discover_contracts(&self) -> Result<Vec<String>, Error> {
        let contracts_dir = self.workspace_path.join("contracts");

        let contracts = std::fs::read_dir(&contracts_dir)?
            .map(|entry_res| -> Result<Option<String>, Error> {
                let entry = entry_res?;
                let path = entry.path();

                // skip non-directories or dirs without Cargo.toml
                let cargo_toml = path.join("Cargo.toml");
                if !path.is_dir() || !cargo_toml.exists() {
                    return Ok(None);
                }

                let content = std::fs::read_to_string(&cargo_toml)?;
                if !content.contains("cdylib") {
                    return Ok(None);
                }

                // parse and extract package.name, propagating any toml errors
                let tv = content.parse::<toml::Value>()?;
                let name = tv
                    .get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| Error::InvalidPackageName)?;

                Ok(Some(name.to_string()))
            })
            .collect::<Result<Vec<Option<String>>, Error>>()? // bubbles up any Err
            .into_iter()
            .flatten()
            .collect();

        Ok(contracts)
    }

    async fn build_contracts(&self) -> Result<(), Error> {
        // Run scaffold build to generate WASM files
        let build_cmd = build::Command {
            build_clients_args: build::clients::Args {
                env: Some(build::clients::ScaffoldEnv::Development),
                workspace_root: Some(self.workspace_path.clone()),
                out_dir: None,
            },
            build: stellar_cli::commands::contract::build::Cmd {
                manifest_path: None,
                package: None,
                profile: "release".to_string(),
                features: None,
                all_features: false,
                no_default_features: false,
                out_dir: None,
                print_commands_only: false,
                meta: Vec::new(),
            },
            list: false,
            build_clients: false, // Don't build clients, just contracts
        };

        build_cmd.run().await?;
        Ok(())
    }

    fn get_constructor_args(&self, contract_name: &str) -> Result<Option<String>, Error> {
        // Get the WASM file path
        let target_dir = self.workspace_path.join("target");
        let wasm_path = stellar_build::stellar_wasm_out_file(&target_dir, contract_name);

        if !wasm_path.exists() {
            return Err(Error::WasmFileNotFound(contract_name.to_string()));
        }

        // Read the WASM file and get spec entries
        let raw_wasm = fs::read(&wasm_path)?;
        let entries = soroban_spec_tools::contract::Spec::new(&raw_wasm)?.spec;
        let spec = soroban_spec_tools::Spec::new(entries.clone());

        // Check if constructor function exists
        let Ok(func) = spec.find_function("__constructor") else {
            return Ok(None);
        };
        if func.inputs.is_empty() {
            return Ok(None);
        }

        // Build the custom command for the constructor
        let cmd = arg_parsing::build_custom_cmd("__constructor", &spec).map_err(|e| {
            Error::ConstructorArgsError(format!("Failed to build constructor command: {e}"))
        })?;

        println!("\n📋 Contract '{contract_name}' requires constructor arguments:");

        let mut args = Vec::new();

        // Loop through the command arguments, skipping file args
        for arg in cmd.get_arguments() {
            let arg_name = arg.get_id().as_str();

            // Skip file arguments (they end with -file-path)
            if arg_name.ends_with("-file-path") {
                continue;
            }

            if let Some(arg_value) = Self::handle_constructor_argument(arg)? {
                args.push(arg_value);
            }
        }

        Ok((!args.is_empty()).then(|| args.join(" ")))
    }

    fn handle_constructor_argument(arg: &clap::Arg) -> Result<Option<String>, Error> {
        let arg_name = arg.get_id().as_str();

        let help_text = arg.get_long_help().or(arg.get_help()).map_or_else(
            || "No description available".to_string(),
            ToString::to_string,
        );

        let value_name = arg
            .get_value_names()
            .map_or_else(|| "VALUE".to_string(), |names| names.join(" "));

        // Display help text before the prompt
        println!("\n  --{arg_name}");
        if value_name != "bool" && !help_text.is_empty() {
            println!("   {help_text}");
        }

        if value_name == "bool" {
            Self::handle_bool_argument(arg_name)
        } else if value_name.contains('|') && Self::is_simple_enum(&value_name) {
            Self::handle_simple_enum_argument(arg_name, &value_name)
        } else {
            // For all other types (complex enums, structs, strings), use string input
            Self::handle_formatted_input(arg_name)
        }
    }

    fn is_simple_enum(value_name: &str) -> bool {
        value_name.split('|').all(|part| {
            let trimmed = part.trim();
            trimmed.parse::<i32>().is_ok() || trimmed.chars().all(|c| c.is_alphabetic() || c == '_')
        })
    }

    fn handle_formatted_input(arg_name: &str) -> Result<Option<String>, Error> {
        let input_result: Result<String, _> = Input::new()
            .with_prompt(format!("Enter value for --{arg_name}"))
            .allow_empty(true)
            .interact();

        let value = input_result
            .as_deref()
            .map(str::trim)
            .map_err(|e| Error::ConstructorArgsError(format!("Input error: {e}")))?;

        let value = if value.is_empty() {
            "# TODO: Fill in value"
        } else {
            // Check if the value is already quoted
            let is_already_quoted = (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''));

            // Only wrap in quotes if it's not already quoted and contains special characters or spaces
            if !is_already_quoted
                && (value.contains(' ')
                    || value.contains('{')
                    || value.contains('[')
                    || value.contains('"'))
            {
                &format!("'{value}'")
            } else {
                value
            }
        };
        Ok(Some(format!("--{arg_name} {value}")))
    }

    fn handle_simple_enum_argument(
        arg_name: &str,
        value_name: &str,
    ) -> Result<Option<String>, Error> {
        let mut select = Select::new()
            .with_prompt(format!("Select value for --{arg_name}"))
            .default(0); // This will show the cursor on the first option initially

        // Add "Skip" option
        select = select.item("(Skip - leave blank)");

        // Parse the values from "a | b | c" format and add numeric options
        let values: Vec<_> = value_name.split('|').collect();
        for value in &values {
            select = select.item(format!("Value: {value}"));
        }

        let selection = select
            .interact()
            .map_err(|e| Error::ConstructorArgsError(format!("Input error: {e}")))?;

        Ok((selection > 0).then(|| {
            // User selected an actual value (not skip)
            let selected_value = values[selection - 1];
            format!("--{arg_name} {selected_value}")
        }))
    }

    fn handle_bool_argument(arg_name: &str) -> Result<Option<String>, Error> {
        let bool_value = Confirm::new()
            .with_prompt(format!("Set --{arg_name} to true?"))
            .default(false)
            .interact()
            .map_err(|e| Error::ConstructorArgsError(format!("Input error: {e}")))?;
        Ok(bool_value.then(|| format!("--{arg_name}")))
    }

    fn setup_env_file(&self) -> Result<(), Error> {
        let env_example_path = self.workspace_path.join(".env.example");
        let env_path = self.workspace_path.join(".env");

        // Only copy if .env.example exists and .env doesn't exist
        if env_example_path.exists() && !env_path.exists() {
            let copy_options = fs_extra::file::CopyOptions::new();
            fs_extra::file::copy(&env_example_path, &env_path, &copy_options)?;
        }

        Ok(())
    }
}
