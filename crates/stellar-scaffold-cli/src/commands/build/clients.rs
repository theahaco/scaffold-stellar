#![allow(clippy::struct_excessive_bools)]
use crate::commands::build::env_toml;
use indexmap::IndexMap;
use regex::Regex;
use serde_json;
use shlex::split;
use std::fmt::Debug;
use std::hash::Hash;
use std::process::Command;
use stellar_cli::commands::NetworkRunnable;
use stellar_cli::utils::contract_hash;
use stellar_cli::{commands as cli, CommandParser};
use stellar_strkey::{self, Contract};
use stellar_xdr::curr::Error as xdrError;

use super::env_toml::Network;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, clap::ValueEnum)]
pub enum ScaffoldEnv {
    Development,
    Testing,
    Staging,
    Production,
}

impl std::fmt::Display for ScaffoldEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

#[derive(clap::Args, Debug, Clone)]
pub struct Args {
    #[arg(env = "STELLAR_ENV", value_enum)]
    pub env: Option<ScaffoldEnv>,
    #[arg(skip)]
    pub workspace_root: Option<std::path::PathBuf>,
    /// Directory where wasm files are located
    #[arg(skip)]
    pub out_dir: Option<std::path::PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    EnvironmentsToml(#[from] env_toml::Error),
    #[error("⛔ ️invalid network: must either specify a network name or both network_passphrase and rpc_url")]
    MalformedNetwork,
    #[error(transparent)]
    ParsingNetwork(#[from] cli::network::Error),
    #[error(transparent)]
    GeneratingKey(#[from] cli::keys::generate::Error),
    #[error("⛔ ️can only have one default account; marked as default: {0:?}")]
    OnlyOneDefaultAccount(Vec<String>),
    #[error("⛔ ️you need to provide at least one account, to use as the source account for contract deployment and other operations")]
    NeedAtLeastOneAccount,
    #[error("⛔ ️No contract named {0:?}")]
    BadContractName(String),
    #[error("⛔ ️Invalid contract ID: {0:?}")]
    InvalidContractID(String),
    #[error("⛔ ️An ID must be set for a contract in production or staging. E.g. <name>.id = C...")]
    MissingContractID(String),
    #[error("⛔ ️Unable to parse script: {0:?}")]
    ScriptParseFailure(String),
    #[error("⛔ ️Failed to execute subcommand: {0:?}\n{1:?}")]
    SubCommandExecutionFailure(String, String),
    #[error(transparent)]
    ContractInstall(#[from] cli::contract::upload::Error),
    #[error(transparent)]
    ContractDeploy(#[from] cli::contract::deploy::wasm::Error),
    #[error(transparent)]
    ContractBindings(#[from] cli::contract::bindings::typescript::Error),
    #[error(transparent)]
    ContractFetch(#[from] cli::contract::fetch::Error),
    #[error(transparent)]
    ConfigLocator(#[from] stellar_cli::config::locator::Error),
    #[error(transparent)]
    ConfigNetwork(#[from] stellar_cli::config::network::Error),
    #[error(transparent)]
    ContractInvoke(#[from] cli::contract::invoke::Error),
    #[error(transparent)]
    Clap(#[from] clap::Error),
    #[error(transparent)]
    WasmHash(#[from] xdrError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("⛔ ️Failed to run npm command in {0:?}: {1:?}")]
    NpmCommandFailure(std::path::PathBuf, String),
}

impl Args {
    pub async fn run(&self, package_names: Vec<String>) -> Result<(), Error> {
        let workspace_root = self
            .workspace_root
            .as_ref()
            .expect("workspace_root must be set before running");

        let Some(current_env) = env_toml::Environment::get(
            workspace_root,
            &self.clone().stellar_scaffold_env(ScaffoldEnv::Production),
        )?
        else {
            return Ok(());
        };

        Self::add_network_to_env(&current_env.network)?;
        // Create the '.stellar' directory if it doesn't exist
        std::fs::create_dir_all(workspace_root.join(".stellar"))
            .map_err(stellar_cli::config::locator::Error::Io)?;
        Self::handle_accounts(current_env.accounts.as_deref()).await?;
        self.clone()
            .handle_contracts(
                current_env.contracts.as_ref(),
                package_names,
                &current_env.network,
            )
            .await?;

        Ok(())
    }

    fn stellar_scaffold_env(self, default: ScaffoldEnv) -> String {
        self.env.unwrap_or(default).to_string().to_lowercase()
    }

    /// Parse the network settings from the environments.toml file and set `STELLAR_RPC_URL` and
    /// `STELLAR_NETWORK_PASSPHRASE`.
    ///
    /// We could set `STELLAR_NETWORK` instead, but when importing contracts, we want to hard-code
    /// the network passphrase. So if given a network name, we use soroban-cli to fetch the RPC url
    /// & passphrase for that named network, and still set the environment variables.
    fn add_network_to_env(network: &env_toml::Network) -> Result<(), Error> {
        match &network {
            Network {
                name: Some(name), ..
            } => {
                let stellar_cli::config::network::Network {
                    rpc_url,
                    network_passphrase,
                    ..
                } = (stellar_cli::config::network::Args {
                    network: Some(name.clone()),
                    rpc_url: None,
                    network_passphrase: None,
                    rpc_headers: Vec::new(),
                })
                .get(&stellar_cli::config::locator::Args {
                    global: false,
                    config_dir: None,
                })?;
                eprintln!("🌐 using {name} network");
                std::env::set_var("STELLAR_RPC_URL", rpc_url);
                std::env::set_var("STELLAR_NETWORK_PASSPHRASE", network_passphrase);
            }
            Network {
                rpc_url: Some(rpc_url),
                network_passphrase: Some(passphrase),
                ..
            } => {
                std::env::set_var("STELLAR_RPC_URL", rpc_url);
                std::env::set_var("STELLAR_NETWORK_PASSPHRASE", passphrase);
                eprintln!("🌐 using network at {rpc_url}");
            }
            _ => return Err(Error::MalformedNetwork),
        }

        Ok(())
    }

    fn get_network_args(network: &Network) -> stellar_cli::config::network::Args {
        stellar_cli::config::network::Args {
            rpc_url: network.rpc_url.clone(),
            network_passphrase: network.network_passphrase.clone(),
            network: network.name.clone(),
            rpc_headers: network.rpc_headers.clone().unwrap_or_default(),
        }
    }

    fn get_config_locator(&self) -> stellar_cli::config::locator::Args {
        let workspace_root = self
            .workspace_root
            .as_ref()
            .expect("workspace_root not set");
        stellar_cli::config::locator::Args {
            global: false,
            config_dir: Some(workspace_root.clone()),
        }
    }

    fn get_contract_alias(
        &self,
        name: &str,
    ) -> Result<Option<Contract>, stellar_cli::config::locator::Error> {
        let config_dir = self.get_config_locator();
        let network_passphrase = std::env::var("STELLAR_NETWORK_PASSPHRASE")
            .expect("No STELLAR_NETWORK_PASSPHRASE environment variable set");
        config_dir.get_contract_id(name, &network_passphrase)
    }

    async fn contract_hash_matches(
        &self,
        contract_id: &Contract,
        hash: &str,
        network: &Network,
    ) -> Result<bool, Error> {
        let result = cli::contract::fetch::Cmd {
            contract_id: stellar_cli::config::UnresolvedContract::Resolved(*contract_id),
            out_file: None,
            locator: self.get_config_locator(),
            network: Self::get_network_args(network),
        }
        .run_against_rpc_server(None, None)
        .await;

        match result {
            Ok(result) => {
                let ctrct_hash = contract_hash(&result)?;
                Ok(hex::encode(ctrct_hash) == hash)
            }
            Err(e) => {
                if e.to_string().contains("Contract not found") {
                    Ok(false)
                } else {
                    Err(Error::ContractFetch(e))
                }
            }
        }
    }

    fn save_contract_alias(
        &self,
        name: &str,
        contract_id: &Contract,
        network: &Network,
    ) -> Result<(), stellar_cli::config::locator::Error> {
        let config_dir = self.get_config_locator();
        let passphrase = network
            .network_passphrase
            .clone()
            .expect("You must set a network passphrase.");
        config_dir.save_contract_id(&passphrase, contract_id, name)
    }

    fn write_contract_template(self, name: &str, contract_id: &str) -> Result<(), Error> {
        let allow_http =
            if self.clone().stellar_scaffold_env(ScaffoldEnv::Production) == "development" {
                "\n  allowHttp: true,"
            } else {
                ""
            };
        let network = std::env::var("STELLAR_NETWORK_PASSPHRASE")
            .expect("No STELLAR_NETWORK_PASSPHRASE environment variable set");
        let template = format!(
            r"import * as Client from '{name}';
import {{ rpcUrl }} from './util';
    
export default new Client.Client({{
  networkPassphrase: '{network}',
  contractId: '{contract_id}',
  rpcUrl,{allow_http}
  publicKey: undefined,
}});
"
        );
        let workspace_root = self
            .workspace_root
            .as_ref()
            .expect("workspace_root not set");
        let path = workspace_root.join(format!("src/contracts/{name}.ts"));
        std::fs::write(path, template)?;
        Ok(())
    }

    async fn generate_contract_bindings(self, name: &str, contract_id: &str) -> Result<(), Error> {
        eprintln!("🎭 binding {name:?} contract");
        let workspace_root = self
            .workspace_root
            .as_ref()
            .expect("workspace_root not set");
        let output_dir = workspace_root.join(format!("packages/{name}"));
        cli::contract::bindings::typescript::Cmd::parse_arg_vec(&[
            "--contract-id",
            contract_id,
            "--output-dir",
            output_dir
                .to_str()
                .expect("we do not support non-utf8 paths"),
            "--overwrite",
        ])?
        .run()
        .await?;

        eprintln!("🍽️ importing {name:?} contract");
        self.write_contract_template(name, contract_id)?;

        // Run `npm i` in the output directory
        eprintln!("🔧 running 'npm install' in {output_dir:?}");
        let output = std::process::Command::new("npm")
            .current_dir(&output_dir)
            .arg("install")
            .arg("--loglevel=error") // Reduce noise from warnings
            .arg("--no-workspaces") // fix issue where stellar sometimes isnt installed locally causing tsc to fail
            .output()?;

        if !output.status.success() {
            return Err(Error::NpmCommandFailure(
                output_dir.clone(),
                format!(
                    "npm install failed with status: {:?}\nError: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        eprintln!("✅ 'npm install' succeeded in {output_dir:?}");

        eprintln!("🔨 running 'npm run build' in {output_dir:?}");
        let output = std::process::Command::new("npm")
            .current_dir(&output_dir)
            .arg("run")
            .arg("build")
            .arg("--loglevel=error") // Reduce noise from warnings
            .output()?;

        if !output.status.success() {
            return Err(Error::NpmCommandFailure(
                output_dir.clone(),
                format!(
                    "npm run build failed with status: {:?}\nError: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        eprintln!("✅ 'npm run build' succeeded in {output_dir:?}");
        Ok(())
    }

    async fn handle_accounts(accounts: Option<&[env_toml::Account]>) -> Result<(), Error> {
        let Some(accounts) = accounts else {
            return Err(Error::NeedAtLeastOneAccount);
        };

        let default_account_candidates = accounts
            .iter()
            .filter(|&account| account.default)
            .map(|account| account.name.clone())
            .collect::<Vec<_>>();

        let default_account = match (default_account_candidates.as_slice(), accounts) {
            ([], []) => return Err(Error::NeedAtLeastOneAccount),
            ([], [env_toml::Account { name, .. }, ..]) => name.clone(),
            ([candidate], _) => candidate.to_string(),
            _ => return Err(Error::OnlyOneDefaultAccount(default_account_candidates)),
        };

        for account in accounts {
            eprintln!("🔐 creating keys for {:?}", account.name);
            cli::keys::generate::Cmd::parse_arg_vec(&[&account.name, "--fund"])?
                .run(&stellar_cli::commands::global::Args::default())
                .await
                .or_else(|e| {
                    if e.to_string().contains("already exists") {
                        // ignore "already exists" errors
                        eprintln!("{e}");
                        Ok(())
                    } else {
                        Err(e)
                    }
                })?;
        }

        std::env::set_var("STELLAR_ACCOUNT", &default_account);

        Ok(())
    }

    fn maintain_user_ordering(
        package_names: &[String],
        contracts: Option<&IndexMap<Box<str>, env_toml::Contract>>,
    ) -> Vec<String> {
        contracts.map_or_else(
            || package_names.to_vec(),
            |contracts| {
                let mut reordered: Vec<String> = contracts
                    .keys()
                    .filter_map(|contract_name| {
                        package_names
                            .iter()
                            .find(|&name| name == contract_name.as_ref())
                            .cloned()
                    })
                    .collect();

                reordered.extend(
                    package_names
                        .iter()
                        .filter(|name| !contracts.contains_key(name.as_str()))
                        .cloned(),
                );

                reordered
            },
        )
    }

    async fn handle_production_contracts(
        &self,
        contracts: &IndexMap<Box<str>, env_toml::Contract>,
    ) -> Result<(), Error> {
        for (name, contract) in contracts.iter().filter(|(_, settings)| settings.client) {
            if let Some(id) = &contract.id {
                if stellar_strkey::Contract::from_string(id).is_err() {
                    return Err(Error::InvalidContractID(id.to_string()));
                }
                self.clone()
                    .generate_contract_bindings(name, &id.to_string())
                    .await?;
            } else {
                return Err(Error::MissingContractID(name.to_string()));
            }
        }
        Ok(())
    }

    async fn handle_contracts(
        self,
        contracts: Option<&IndexMap<Box<str>, env_toml::Contract>>,
        package_names: Vec<String>,
        network: &Network,
    ) -> Result<(), Error> {
        if package_names.is_empty() {
            return Ok(());
        }

        let env = self.clone().stellar_scaffold_env(ScaffoldEnv::Production);
        if env == "production" || env == "staging" {
            if let Some(contracts) = contracts {
                self.handle_production_contracts(contracts).await?;
            }
            return Ok(());
        }

        self.validate_contract_names(contracts)?;

        let names = Self::maintain_user_ordering(&package_names, contracts);
        for name in names {
            let settings = contracts
                .and_then(|contracts| contracts.get(name.as_str()))
                .cloned()
                .unwrap_or_default();

            // Skip if client generation is disabled
            if !settings.client {
                continue;
            }

            self.process_single_contract(&name, settings, network, &env)
                .await?;
        }

        Ok(())
    }

    fn get_wasm_path(&self, contract_name: &str) -> std::path::PathBuf {
        // Check if out_dir was specified and use it, otherwise fall back to target directory
        if let Some(out_dir) = &self.out_dir {
            out_dir.join(format!("{contract_name}.wasm"))
        } else {
            let workspace_root = self
                .workspace_root
                .as_ref()
                .expect("workspace_root not set");
            let target_dir = workspace_root.join("target");
            stellar_build::stellar_wasm_out_file(&target_dir, contract_name)
        }
    }

    fn validate_contract_names(
        &self,
        contracts: Option<&IndexMap<Box<str>, env_toml::Contract>>,
    ) -> Result<(), Error> {
        if let Some(contracts) = contracts {
            for (name, _) in contracts.iter().filter(|(_, settings)| settings.client) {
                let wasm_path = self.get_wasm_path(name);
                if !wasm_path.exists() {
                    return Err(Error::BadContractName(name.to_string()));
                }
            }
        }
        Ok(())
    }

    async fn process_single_contract(
        &self,
        name: &str,
        settings: env_toml::Contract,
        network: &Network,
        env: &str,
    ) -> Result<(), Error> {
        // First check if we have an ID in settings
        let contract_id = if let Some(id) = &settings.id {
            Contract::from_string(id).map_err(|_| Error::InvalidContractID(id.clone()))?
        } else {
            let wasm_path = self.get_wasm_path(name);
            if !wasm_path.exists() {
                return Err(Error::BadContractName(name.to_string()));
            }

            let hash = self.upload_contract_wasm(name, &wasm_path).await?;

            // Check existing alias - if it exists and matches hash, we can return early
            if let Some(existing_contract_id) = self.get_contract_alias(name)? {
                if self
                    .contract_hash_matches(&existing_contract_id, &hash, network)
                    .await?
                {
                    eprintln!("✅ Contract {name:?} is up to date");
                    return Ok(());
                }
                eprintln!("🔄 Updating contract {name:?}");
            }

            // Deploy new contract if we got here
            let contract_id = self.deploy_contract(name, &hash, &settings).await?;
            self.save_contract_alias(name, &contract_id, network)?;
            contract_id
        };

        // Run after_deploy script if in development or test environment
        if (env == "development" || env == "testing") && settings.after_deploy.is_some() {
            eprintln!("🚀 Running after_deploy script for {name:?}");
            self.run_after_deploy_script(
                name,
                &contract_id,
                settings.after_deploy.as_ref().unwrap(),
            )
            .await?;
        }

        self.clone()
            .generate_contract_bindings(name, &contract_id.to_string())
            .await?;

        Ok(())
    }

    async fn upload_contract_wasm(
        &self,
        name: &str,
        wasm_path: &std::path::Path,
    ) -> Result<String, Error> {
        eprintln!("📲 installing {name:?} wasm bytecode on-chain...");
        let hash = cli::contract::upload::Cmd::parse_arg_vec(&[
            "--wasm",
            wasm_path
                .to_str()
                .expect("we do not support non-utf8 paths"),
        ])?
        .run_against_rpc_server(None, None)
        .await?
        .into_result()
        .expect("no hash returned by 'contract upload'")
        .to_string();
        eprintln!("    ↳ hash: {hash}");
        Ok(hash)
    }

    fn parse_script_line(line: &str) -> Result<(Option<String>, Vec<String>), Error> {
        let re = Regex::new(r"\$\((.*?)\)").expect("Invalid regex pattern");
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let resolved_line = Self::resolve_line(&re, line, shell, flag)?;
        let parts = split(&resolved_line)
            .ok_or_else(|| Error::ScriptParseFailure(resolved_line.to_string()))?;

        let (source_account, command_parts): (Vec<_>, Vec<_>) = parts
            .iter()
            .partition(|&part| part.starts_with("STELLAR_ACCOUNT="));

        let source = source_account.first().map(|account| {
            account
                .strip_prefix("STELLAR_ACCOUNT=")
                .unwrap()
                .to_string()
        });

        Ok((
            source,
            command_parts.iter().map(|s| (*s).to_string()).collect(),
        ))
    }

    async fn deploy_contract(
        &self,
        name: &str,
        hash: &str,
        settings: &env_toml::Contract,
    ) -> Result<Contract, Error> {
        let mut deploy_args = vec![
            "--alias".to_string(),
            name.to_string(),
            "--wasm-hash".to_string(),
            hash.to_string(),
        ];

        if let Some(constructor_script) = &settings.constructor_args {
            let (source_account, mut args) = Self::parse_script_line(constructor_script)?;

            if let Some(account) = source_account {
                deploy_args.extend_from_slice(&["--source-account".to_string(), account]);
            }

            deploy_args.push("--".to_string());
            deploy_args.append(&mut args);
        }

        eprintln!("🪞 instantiating {name:?} smart contract");
        let deploy_arg_refs: Vec<&str> = deploy_args
            .iter()
            .map(std::string::String::as_str)
            .collect();
        let contract_id = cli::contract::deploy::wasm::Cmd::parse_arg_vec(&deploy_arg_refs)?
            .run_against_rpc_server(None, None)
            .await?
            .into_result()
            .expect("no contract id returned by 'contract deploy'");
        eprintln!("    ↳ contract_id: {contract_id}");

        Ok(contract_id)
    }

    fn resolve_line(re: &Regex, line: &str, shell: &str, flag: &str) -> Result<String, Error> {
        let mut result = String::new();
        let mut last_match = 0;
        for cap in re.captures_iter(line) {
            let whole_match = cap.get(0).unwrap();
            result.push_str(&line[last_match..whole_match.start()]);
            let cmd = &cap[1];
            let output = Self::execute_subcommand(shell, flag, cmd)?;
            result.push_str(&output);
            last_match = whole_match.end();
        }
        result.push_str(&line[last_match..]);
        Ok(result)
    }

    fn execute_subcommand(shell: &str, flag: &str, cmd: &str) -> Result<String, Error> {
        match Command::new(shell).arg(flag).arg(cmd).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                if output.status.success() {
                    Ok(stdout)
                } else {
                    Err(Error::SubCommandExecutionFailure(cmd.to_string(), stderr))
                }
            }
            Err(e) => Err(Error::SubCommandExecutionFailure(
                cmd.to_string(),
                e.to_string(),
            )),
        }
    }

    async fn run_after_deploy_script(
        &self,
        name: &str,
        contract_id: &Contract,
        after_deploy_script: &str,
    ) -> Result<(), Error> {
        for line in after_deploy_script.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let (source_account, command_parts) = Self::parse_script_line(line)?;

            let contract_id_arg = contract_id.to_string();
            let mut args = vec!["--id", &contract_id_arg];
            if let Some(account) = source_account.as_ref() {
                args.extend_from_slice(&["--source-account", account]);
            }
            args.extend_from_slice(&["--"]);
            args.extend(command_parts.iter().map(std::string::String::as_str));

            eprintln!("  ↳ Executing: stellar contract invoke {}", args.join(" "));
            let result = cli::contract::invoke::Cmd::parse_arg_vec(&args)?
                .run_against_rpc_server(None, None)
                .await?;
            eprintln!("  ↳ Result: {result:?}");
        }
        eprintln!("✅ After deploy script for {name:?} completed successfully");
        Ok(())
    }
}
