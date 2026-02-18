use clap::Parser;
use flate2::read::GzDecoder;
use reqwest;
use serde::Deserialize;
use std::{fs, path::Path};
use stellar_cli::commands::global;
use stellar_cli::print::Print;
use tar::Archive;
use dialoguer::{Input, Select, theme::ColorfulTheme, MultiSelect, Confirm};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

#[derive(Parser, Debug)]
pub struct Cmd {
    /// Clone contract from `OpenZeppelin` examples
    #[arg(long, conflicts_with_all = ["ls", "from_wizard"])]
    pub from: Option<String>,

    /// List available contract examples
    #[arg(long, conflicts_with_all = ["from", "from_wizard"])]
    pub ls: bool,

    /// Open contract generation wizard in browser
    #[arg(long, conflicts_with_all = ["from", "ls"])]
    pub from_wizard: bool,

    /// Open contract generation wizard in cli
    #[arg(long, conflicts_with_all = ["ls", "from_wizard", "from"])]
    pub from_cli: bool,

    /// Output directory for the generated contract (defaults to contracts/<example-name>)
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    CargoToml(#[from] cargo_toml::Error),
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Git command failed: {0}")]
    GitCloneFailed(String),
    #[error("Example '{0}' not found in OpenZeppelin stellar-contracts")]
    ExampleNotFound(String),
    #[error("Failed to open browser: {0}")]
    BrowserFailed(String),
    #[error("Wrong input")]
    WrongInput(String),
    #[error("File not found")]
    FileNotFound(String),
    #[error("Config Error")]
    ConfigError(String),
    #[error("Contract creation failed")]
    ContractCreationFailed(String),
    #[error("No action specified. Use --from, --ls, or --from-wizard")]
    NoActionSpecified,
}

impl Cmd {
    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        match (&self.from, self.ls, self.from_wizard, self.from_cli) {
            (Some(example_name), _, _, _) => self.clone_example(example_name, global_args).await,
            (_, true, _, _) => self.list_examples(global_args).await,
            (_, _, true, _) => open_wizard(global_args),
            (_, _, _, true) => open_wizard_cli(global_args),
            _ => Err(Error::NoActionSpecified),
        }
    }

    async fn clone_example(
        &self,
        example_name: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        printer.infoln(format!("Downloading example '{example_name}'..."));

        let dest_path = self
            .output
            .clone()
            .unwrap_or_else(|| format!("contracts/{example_name}"));

        let repo_cache_path = self.ensure_cache_updated().await?;

        // Check if the example exists
        let example_source_path = repo_cache_path.join(format!("examples/{example_name}"));
        if !example_source_path.exists() {
            return Err(Error::ExampleNotFound(example_name.to_string()));
        }

        // Create destination and copy example contents
        fs::create_dir_all(&dest_path)?;
        Self::copy_directory_contents(&example_source_path, Path::new(&dest_path))?;

        // Get the latest release tag we're using
        let Release { tag_name } = Self::fetch_latest_release().await?;

        // Read and update workspace Cargo.toml
        let workspace_cargo_path = Path::new("Cargo.toml");
        if workspace_cargo_path.exists() {
            Self::update_workspace_dependencies(
                workspace_cargo_path,
                &example_source_path,
                &tag_name,
                global_args,
            )?;
        } else {
            printer.warnln("Warning: No workspace Cargo.toml found in current directory.");
            printer
                .println("   You'll need to manually add required dependencies to your workspace.");
        }

        printer.checkln(format!(
            "Successfully downloaded example '{example_name}' to {dest_path}"
        ));
        printer
            .infoln("You may need to modify your environments.toml to add constructor arguments!");
        Ok(())
    }

    fn update_workspace_dependencies(
        workspace_path: &Path,
        example_path: &Path,
        tag: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        let example_cargo_content = fs::read_to_string(example_path.join("Cargo.toml"))?;
        let deps = Self::extract_stellar_dependencies(&example_cargo_content)?;
        if deps.is_empty() {
            return Ok(());
        }

        // Parse the workspace Cargo.toml
        let mut manifest = cargo_toml::Manifest::from_path(workspace_path)?;

        // Ensure workspace.dependencies exists
        if manifest.workspace.is_none() {
            // Create a minimal workspace with just what we need
            let workspace_toml = r"
[workspace]
members = []

[workspace.dependencies]
";
            let workspace: cargo_toml::Workspace<toml::Value> = toml::from_str(workspace_toml)?;
            manifest.workspace = Some(workspace);
        }
        let workspace = manifest.workspace.as_mut().unwrap();

        let mut workspace_deps = workspace.dependencies.clone();

        let mut added_deps = Vec::new();
        let mut updated_deps = Vec::new();

        for dep in deps {
            let git_dep = cargo_toml::DependencyDetail {
                git: Some("https://github.com/OpenZeppelin/stellar-contracts".to_string()),
                tag: Some(tag.to_string()),
                ..Default::default()
            };

            if let Some(existing_dep) = workspace_deps.clone().get(&dep) {
                // Check if we need to update the tag
                if let cargo_toml::Dependency::Detailed(detail) = existing_dep
                    && let Some(existing_tag) = &detail.tag
                    && existing_tag != tag
                {
                    workspace_deps.insert(
                        dep.clone(),
                        cargo_toml::Dependency::Detailed(Box::new(git_dep)),
                    );
                    updated_deps.push((dep, existing_tag.clone()));
                }
            } else {
                workspace_deps.insert(
                    dep.clone(),
                    cargo_toml::Dependency::Detailed(Box::new(git_dep)),
                );
                added_deps.push(dep);
            }
        }

        if !added_deps.is_empty() || !updated_deps.is_empty() {
            workspace.dependencies = workspace_deps;
            // Write the updated manifest back to file
            let toml_string = toml::to_string_pretty(&manifest)?;
            fs::write(workspace_path, toml_string)?;

            if !added_deps.is_empty() {
                printer.infoln("Added the following dependencies to workspace:");
                for dep in added_deps {
                    printer.println(format!("   • {dep}"));
                }
            }

            if !updated_deps.is_empty() {
                printer.infoln("Updated the following dependencies:");
                for (dep, old_tag) in updated_deps {
                    printer.println(format!("   • {dep}: {old_tag} -> {tag}"));
                }
            }
        }

        Ok(())
    }

    fn extract_stellar_dependencies(cargo_toml_content: &str) -> Result<Vec<String>, Error> {
        let manifest: cargo_toml::Manifest = toml::from_str(cargo_toml_content)?;

        Ok(manifest
            .dependencies
            .iter()
            .filter(|(dep_name, _)| dep_name.starts_with("stellar-"))
            .filter_map(|(dep_name, dep_detail)| match dep_detail {
                cargo_toml::Dependency::Detailed(detail)
                    if !(detail.inherited || detail.git.is_some()) =>
                {
                    None
                }
                _ => Some(dep_name.clone()),
            })
            .collect())
    }

    async fn list_examples(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        printer.infoln("Fetching available contract examples...");

        let repo_cache_path = self.ensure_cache_updated().await?;
        let examples_path = repo_cache_path.join("examples");

        let mut examples: Vec<String> = if examples_path.exists() {
            fs::read_dir(examples_path)?
                .filter_map(std::result::Result::ok)
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(std::string::ToString::to_string)
                })
                .collect()
        } else {
            Vec::new()
        };

        examples.sort();

        printer.println("\nAvailable contract examples:");
        printer.println("────────────────────────────────");

        for example in &examples {
            printer.println(format!("  📁 {example}"));
        }

        printer.println("\nUsage:");
        printer.println("   stellar-scaffold contract generate --from <example-name>");
        printer.println("   Example: stellar-scaffold contract generate --from nft-royalties");

        Ok(())
    }

    async fn fetch_latest_release() -> Result<Release, Error> {
        Self::fetch_latest_release_from_url(
            "https://api.github.com/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .await
    }

    async fn fetch_latest_release_from_url(url: &str) -> Result<Release, Error> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "stellar-scaffold-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
        }

        let release: Release = response.json().await?;
        Ok(release)
    }

    async fn cache_repository(repo_cache_path: &Path, tag_name: &str) -> Result<(), Error> {
        fs::create_dir_all(repo_cache_path)?;

        // Download and extract the specific tag directly
        Self::download_and_extract_tag(repo_cache_path, tag_name).await?;

        if repo_cache_path.read_dir()?.next().is_none() {
            return Err(Error::GitCloneFailed(format!(
                "Failed to download repository release {tag_name} to cache"
            )));
        }

        Ok(())
    }

    async fn download_and_extract_tag(dest_path: &Path, tag_name: &str) -> Result<(), Error> {
        let url =
            format!("https://github.com/OpenZeppelin/stellar-contracts/archive/{tag_name}.tar.gz",);

        // Download the tar.gz file
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "stellar-scaffold-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::GitCloneFailed(format!(
                "Failed to download release {tag_name}: HTTP {}",
                response.status()
            )));
        }

        // Get the response bytes
        let bytes = response.bytes().await?;

        // Extract the tar.gz in a blocking task to avoid blocking the async runtime
        let dest_path = dest_path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let tar = GzDecoder::new(std::io::Cursor::new(bytes));
            let mut archive = Archive::new(tar);

            for entry in archive.entries()? {
                let mut entry = entry?;
                let path = entry.path()?;

                // Strip the root directory (stellar-contracts-{tag}/)
                let stripped_path = path.components().skip(1).collect::<std::path::PathBuf>();

                if stripped_path.as_os_str().is_empty() {
                    continue;
                }

                let dest_file_path = dest_path.join(&stripped_path);

                if entry.header().entry_type().is_dir() {
                    std::fs::create_dir_all(&dest_file_path)?;
                } else {
                    if let Some(parent) = dest_file_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&dest_file_path)?;
                }
            }

            Ok::<(), std::io::Error>(())
        })
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?
        .map_err(Error::Io)?;

        Ok(())
    }

    async fn ensure_cache_updated(&self) -> Result<std::path::PathBuf, Error> {
        let cache_dir = dirs::cache_dir().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache directory not found",
            ))
        })?;

        let base_cache_path = cache_dir.join("stellar-scaffold-cli/openzeppelin-stellar-contracts");

        // Get the latest release tag
        let Release { tag_name } = Self::fetch_latest_release().await?;
        let repo_cache_path = base_cache_path.join(&tag_name);
        if !repo_cache_path.exists() {
            Self::cache_repository(&repo_cache_path, &tag_name).await?;
        }

        Ok(repo_cache_path)
    }

    fn copy_directory_contents(source: &Path, dest: &Path) -> Result<(), Error> {
        let copy_options = fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true);

        fs_extra::dir::copy(source, dest, &copy_options)
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        Ok(())
    }
}
fn open_wizard_cli(global_args: &global::Args) -> Result<(), Error> {
    let printer = Print::new(global_args.quiet);
    
    // Load the wizard JSON configuration
    let wizard_config = load_wizard_config()?;
    
    printer.println("Welcome to the Stellar Smart Contract Wizard!");
    printer.println("");
    
    // Step 1: Token Type Selection
    let token_types = ["Fungible", "Non-Fungible", "Stablecoin"];
    let token_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What type of contract do you want to generate?")
        .items(&token_types)
        .default(0)
        .interact()
        .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
    
    let token_type = token_types[token_selection];

    // Step 2: Basic Metadata
    let name: String = Input::new()
        .with_prompt("Contract name")
        .default("MyToken".into())
        .validate_with(|input: &String| {
            if input.is_empty() {
                Err("Name can't be empty")
            } else if !input.chars().next().unwrap().is_alphabetic() {
                Err("Name must start with a letter")
            } else if !input.chars().all(|c| c.is_alphanumeric() || c == '_') {
                Err("Name can only contain letters, numbers, and underscores")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;

    let symbol: String = Input::new()
        .with_prompt("Token symbol")
        .default("MTK".into())
        .validate_with(|input: &String| {
            if input.is_empty() {
                Err("Symbol can't be empty")
            } else if input.len() > 10 {
                Err("Symbol should be 10 characters or less")
            } else {
                Ok(())
            }
        })
        .interact_text()
        .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;

    // Step 3: Premint (Fungible and Stablecoin only)
    let premint = if token_type != "Non-Fungible" {
        let premint_str: String = Input::new()
            .with_prompt("Initial supply to premint (0 for none)")
            .default("0".into())
            .validate_with(|input: &String| {
                match input.parse::<u128>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Must be a valid non-negative number"),
                }
            })
            .interact_text()
            .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
        
        let amount: u128 = premint_str.parse().unwrap_or(0);
        if amount > 0 { Some(premint_str) } else { None }
    } else {
        None
    };

    // Step 3b: URI for Non-Fungible tokens
    let uri = if token_type == "Non-Fungible" {
        Some(Input::new()
            .with_prompt("Base URI for token metadata")
            .default("https://example.com/metadata/".into())
            .validate_with(|input: &String| {
                if input.is_empty() {
                    Err("URI can't be empty")
                } else if !input.starts_with("http://") && !input.starts_with("https://") && !input.starts_with("ipfs://") {
                    Err("URI should start with http://, https://, or ipfs://")
                } else {
                    Ok(())
                }
            })
            .interact_text()
            .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?
)
    } else {
        None
    };

    // Step 4: NFT Variant Selection
    let nft_variant = if token_type == "Non-Fungible" {
        let variant_options = ["Base", "Enumerable", "Consecutive"];
        let variant_descriptions = [
            "Base - Standard NFT implementation",
            "Enumerable - Track all tokens on-chain",
            "Consecutive - Optimized for batch minting",
        ];
        
        let variant_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select NFT implementation variant")
            .items(&variant_descriptions)
            .default(0)
            .interact()
            .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
        
        Some(variant_options[variant_selection].to_string())
    } else {
        None
    };

    // Step 5: Features Selection
    let mut selected_features = Vec::new();
    
    let feature_options: &[&str] = if token_type == "Non-Fungible" {
        &["Burnable", "Pausable", "Upgradeable"]
    } else {
        &["Mintable", "Burnable", "Pausable", "Upgradeable"]
    };
    
    let feature_defaults: Vec<bool> = vec![false; feature_options.len()];
    
    let feature_selections: Vec<usize> = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select features to include (Space to toggle, Enter to confirm)")
        .items(feature_options)
        .defaults(&feature_defaults)
        .interact()
        .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;

    if !feature_selections.is_empty() {
        printer.println("\nSelected features:");
    }
    
    for idx in &feature_selections {
        selected_features.push(feature_options[*idx].to_lowercase());
        printer.println(format!("  ✓ {}", feature_options[*idx]));
    }
    
    // Add NFT variant for code generation
    if token_type == "Non-Fungible" {
        if let Some(ref variant) = nft_variant {
            if variant != "Base" {
                selected_features.push(variant.to_lowercase());
            }
        }
    }

    // Step 6: Limitation (Stablecoin only)
    let limitation = if token_type == "Stablecoin" {
        let limitation_descriptions = [
            "None - No transfer restrictions",
            "Allowlist - Only allowlisted addresses can transfer tokens",
            "Blocklist - Block specific addresses from transferring tokens",
        ];
        
        let limitation_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select transfer limitation")
            .items(&limitation_descriptions)
            .default(0)
            .interact()
            .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
        
        match limitation_selection {
            1 => {
                printer.println("  ✓ Allowlist");
                Some("allowlist".to_string())
            }
            2 => {
                printer.println("  ✓ Blocklist");
                Some("blocklist".to_string())
            }
            _ => None,
        }
    } else {
        None
    };

    // Step 7: NFT Minting
    let nft_minting = if token_type == "Non-Fungible" {
        let variant = nft_variant.as_ref().map(|s| s.as_str()).unwrap_or("Base");
        if variant != "Consecutive" {
            let add_minting = Confirm::new()
                .with_prompt("Do you want to add a minting function?")
                .default(true)
                .interact()
                .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
            
            if add_minting {
                let mint_types = ["Sequential", "Non-Sequential"];
                let mint_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select minting type")
                    .items(&mint_types)
                    .default(0)
                    .interact()
                    .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
                
                if mint_selection == 0 {
                    Some("sequential".to_string())
                } else {
                    Some("non_sequential".to_string())
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Step 8: Access Control Selection
    let requires_access_control = selected_features.iter().any(|f| {
        f == "mintable" || f == "pausable" || f == "upgradeable"
    }) || limitation.is_some();
    
    let access_control = if requires_access_control {
        printer.println("\nAccess control is required for the selected features");
        
        let forced_options = ["Ownable", "Roles"];
        let access_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select access control method")
            .items(&forced_options)
            .default(0)
            .interact()
            .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
        
        forced_options[access_selection].to_string()
    } else {
        let access_control_options = ["Ownable", "Roles", "None"];
        let access_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select access control method")
            .items(&access_control_options)
            .default(2)
            .interact()
            .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
        
        access_control_options[access_selection].to_string()
    };

    // Step 9: Summary
    printer.println("\n╔════════════════════════════════════════╗");
    printer.println("║   Contract Configuration Summary       ║");
    printer.println("╚════════════════════════════════════════╝");
    printer.println(format!("  Name:             {}", name));
    printer.println(format!("  Symbol:           {}", symbol));
    if let Some(ref premint_val) = premint {
        printer.println(format!("  Premint:          {}", premint_val));
    }
    printer.println(format!("  Type:             {}", token_type));
    
    if let Some(ref uri_val) = uri {
        printer.println(format!("  Base URI:         {}", uri_val));
    }
    
    if let Some(ref variant) = nft_variant {
        printer.println(format!("  NFT Variant:      {}", variant));
    }
    
    if let Some(ref mint_type) = nft_minting {
        let display = if mint_type == "sequential" { "Sequential" } else { "Non-Sequential" };
        printer.println(format!("  NFT Minting:      {}", display));
    }
    
    if selected_features.is_empty() {
        printer.println("  Features:         (none)");
    } else {
        let display_features: Vec<String> = selected_features.iter()
            .filter(|f| *f != "enumerable" && *f != "consecutive")
            .map(|f| f.clone())
            .collect();
        printer.println(format!("  Features:         {}", display_features.join(", ")));
    }
    
    if let Some(ref lim) = limitation {
        let display = match lim.as_str() {
            "allowlist" => "Allowlist",
            "blocklist" => "Blocklist",
            _ => lim.as_str(),
        };
        printer.println(format!("  Limitation:       {}", display));
    }
    
    printer.println(format!("  Access Control:   {}", access_control));
    
    printer.println("");

    // Step 10: Confirmation
    let confirm = Confirm::new()
        .with_prompt("Proceed with contract generation?")
        .default(true)
        .interact()
        .map_err(|e| Error::WrongInput(format!("Please enter a valid input: {e}")))?;
    
    if !confirm {
        printer.println("\n❌ Contract generation cancelled.");
        return Ok(());
    }

    // Step 11: Generate the contract
    printer.println("\n⚙️  Generating contract...");
    
    let contract_config = ContractConfig {
        name: name.clone(),
        symbol,
        token_type: token_type.to_string(),
        premint,
        uri,
        nft_variant,
        nft_minting,
        features: selected_features,
        access_control,
        limitation,
    };

    let contract_code = generate_contract(&wizard_config, &contract_config)?;
    
    // Step 12: Save the contract
    let output_path = format!("contracts/{}.rs", contract_config.name.to_lowercase());
    save_contract(&output_path, &contract_code)?;
    
    printer.println(format!("\n✅ Contract generated successfully!"));
    printer.println(format!("   Location: {}", output_path));
    printer.println(format!("\n💡 Next steps:"));
    printer.println("   1. Build with: stellar contract build");
    printer.println("   2. Deploy with: stellar contract deploy\n");

    Ok(())
}

// Helper struct to hold contract configuration
#[derive(Debug)]
struct ContractConfig {
    name: String,
    symbol: String,
    token_type: String,
    premint: Option<String>,
    uri: Option<String>,
    nft_variant: Option<String>,
    nft_minting: Option<String>,
    features: Vec<String>,
    access_control: String,
    limitation: Option<String>,
}

// Load the wizard configuration from JSON
fn load_wizard_config() -> Result<Value, Error> {
    let config_str = include_str!("wizard_config.json");
    serde_json::from_str(config_str)
        .map_err(|e| Error::ConfigError(format!("Failed to parse wizard config: {}", e)))
}

// Generate the contract code from the configuration
fn generate_contract(wizard_config: &Value, config: &ContractConfig) -> Result<String, Error> {
    let mut contract = String::new();
    
    // 1. Add license header
    if let Some(license) = wizard_config["wizard"]["license"].as_array() {
        for line in license {
            contract.push_str(line.as_str().unwrap_or(""));
            contract.push('\n');
        }
        contract.push('\n');
    }
    
    // 2. Add imports
    contract.push_str(&generate_imports(wizard_config, config)?);
    contract.push('\n');
    
    // 3. Add contract struct
    contract.push_str("#[contract]\n");
    contract.push_str(&format!("pub struct {} {{}}\n\n", config.name));
    
    // 4. Add main contractimpl block
    contract.push_str(&generate_main_contractimpl(wizard_config, config)?);
    contract.push('\n');
    
    // 5. Add token trait implementation
    contract.push_str(&generate_token_trait_implementation(wizard_config, config)?);
    contract.push('\n');
    
    // 6. Add feature trait extensions
    contract.push_str(&generate_feature_extensions(wizard_config, config)?);
    
    // 7. Add utility functions (access control, pausable, etc.)
    contract.push_str(&generate_utils(wizard_config, config)?);
    
    // Final pass: replace all remaining name placeholders.
    let contract = contract.replace("<NAME>", &config.name);
    
    Ok(contract)
}

// Merge import statements that share the same module path
fn merge_imports(imports: Vec<String>) -> Vec<String> {
    use std::collections::BTreeMap;
    
    // Map from module path to list of imported items
    let mut module_items: BTreeMap<String, Vec<String>> = BTreeMap::new();
    
    for import in &imports {
        let trimmed = import.trim();
        if !trimmed.starts_with("use ") || !trimmed.ends_with(';') {
            continue;
        }
        
        // Strip "use " prefix and ";" suffix
        let inner = &trimmed[4..trimmed.len() - 1];
        
        if let Some(brace_start) = inner.find("::{") {
            // Grouped import
            let module_path = &inner[..brace_start];
            let items_str = &inner[brace_start + 3..inner.len() - 1];
            let items: Vec<String> = items_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            module_items
                .entry(module_path.to_string())
                .or_default()
                .extend(items);
        } else if let Some(last_sep) = inner.rfind("::") {
            // Single import like `stellar_macros::only_owner`
            let module_path = &inner[..last_sep];
            let item = &inner[last_sep + 2..];
            module_items
                .entry(module_path.to_string())
                .or_default()
                .push(item.to_string());
        }
    }
    
    // Merge child module paths into their parent when the parent exists.
    let mut paths: Vec<String> = module_items.keys().cloned().collect();
    paths.sort_by(|a, b| b.len().cmp(&a.len()));
    
    for child_path in &paths {
        if !module_items.contains_key(child_path.as_str()) {
            continue;
        }
        
        // Find the closest parent path that exists in module_items
        let mut best_parent: Option<String> = None;
        for candidate in module_items.keys() {
            if candidate == child_path {
                continue;
            }
            if child_path.starts_with(candidate.as_str())
                && child_path[candidate.len()..].starts_with("::")
            {
                if best_parent.as_ref().map_or(true, |bp| candidate.len() > bp.len()) {
                    best_parent = Some(candidate.clone());
                }
            }
        }
        
        if let Some(parent_path) = best_parent {
            let suffix = &child_path[parent_path.len() + 2..]; // strip the "::"
            let child_items = module_items.remove(child_path.as_str()).unwrap();
            let parent_items = module_items.get_mut(parent_path.as_str()).unwrap();
            for item in child_items {
                parent_items.push(format!("{suffix}::{item}"));
            }
        }
    }
    
    let mut result = Vec::new();
    
    for (module_path, items) in &module_items {
        // Deduplicate items
        let mut seen = HashSet::new();
        let mut unique_items: Vec<String> = Vec::new();
        for item in items {
            if seen.insert(item.clone()) {
                unique_items.push(item.clone());
            }
        }
        
        // Sort with `self` / `self as X` first, then alphabetical
        unique_items.sort_by(|a, b| {
            let a_is_self = a == "self" || a.starts_with("self as");
            let b_is_self = b == "self" || b.starts_with("self as");
            match (a_is_self, b_is_self) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });
        
        if unique_items.len() == 1 {
            result.push(format!("use {}::{};", module_path, unique_items[0]));
        } else {
            result.push(format!("use {}::{{{}}};", module_path, unique_items.join(", ")));
        }
    }
    
    result.sort();
    result
}

// Generate imports based on features
fn generate_imports(wizard_config: &Value, config: &ContractConfig) -> Result<String, Error> {
    let mut imports = HashSet::new();
    
    // Base imports - always included
    imports.insert("use soroban_sdk::{contract, contractimpl, Env, String, Symbol, Address};".to_string());
    
    // Token type specific imports
    let is_nft = config.token_type == "Non-Fungible";
    
    if is_nft {
        // Determine which base type to import based on variant
        let variant = config.nft_variant.as_ref().map(|s| s.as_str()).unwrap_or("Base");
        match variant {
            "Enumerable" => {
                imports.insert("use stellar_tokens::non_fungible::{NonFungibleToken, enumerable::Enumerable};".to_string());
            }
            "Consecutive" => {
                imports.insert("use stellar_tokens::non_fungible::{NonFungibleToken, consecutive::Consecutive};".to_string());
            }
            _ => {
                imports.insert("use stellar_tokens::non_fungible::{NonFungibleToken, Base};".to_string());
            }
        }
    } else {
        // Check for stablecoin limitation first, then regular allowlist/blocklist features
        if let Some(ref lim) = config.limitation {
            // Stablecoin with limitation
            imports.insert("use stellar_tokens::fungible::{FungibleToken, Base};".to_string());
            match lim.as_str() {
                "allowlist" => {
                    imports.insert("use stellar_tokens::fungible::allowlist::{AllowList, FungibleAllowList};".to_string());
                }
                "blocklist" => {
                    imports.insert("use stellar_tokens::fungible::blocklist::{BlockList, FungibleBlockList};".to_string());
                }
                _ => {}
            }
        } else {
            imports.insert("use stellar_tokens::fungible::{FungibleToken, Base};".to_string());
        }
    }
    
    // Access control imports
    if config.access_control == "Ownable" {
        if let Some(ownable_imports) = wizard_config["wizard"]["access_control"]["ownable"]["imports"].as_array() {
            for import in ownable_imports {
                if let Some(import_str) = import.as_str() {
                    imports.insert(import_str.to_string());
                }
            }
        }
    } else if config.access_control == "Roles" {
        if let Some(roles_imports) = wizard_config["wizard"]["access_control"]["roles"]["imports"].as_array() {
            for import in roles_imports {
                if let Some(import_str) = import.as_str() {
                    imports.insert(import_str.to_string());
                }
            }
        }
    }
    
    // Limitation-specific imports from config
    if let Some(ref lim) = config.limitation {
        if let Some(lim_imports) = wizard_config["wizard"]["limitations"][lim.as_str()]["imports"].as_array() {
            for import in lim_imports {
                if let Some(import_str) = import.as_str() {
                    imports.insert(import_str.to_string());
                }
            }
        }
    }
    
    // Feature-specific imports
    for feature in &config.features {
        // Get the appropriate imports key
        let imports_key = if is_nft && feature == "burnable" {
            "imports_nft"
        } else {
            "imports"
        };
        
        if let Some(feature_imports) = wizard_config["wizard"]["features"][feature][imports_key].as_array() {
            for import in feature_imports {
                if let Some(import_str) = import.as_str() {
                    imports.insert(import_str.to_string());
                }
            }
        }
        
        // Fallback to regular imports if NFT-specific not found
        if is_nft && imports_key == "imports_nft" {
            if let Some(feature_imports) = wizard_config["wizard"]["features"][feature]["imports"].as_array() {
                for import in feature_imports {
                    if let Some(import_str) = import.as_str() {
                        imports.insert(import_str.to_string());
                    }
                }
            }
        }
    }
    
    // Convert set to sorted vector, merge common module paths, then output
    let import_vec: Vec<String> = imports.into_iter().collect();
    let merged = merge_imports(import_vec);
    
    Ok(merged.join("\n"))
}

// Generate main contractimpl block
fn generate_main_contractimpl(wizard_config: &Value, config: &ContractConfig) -> Result<String, Error> {
    let mut code = String::new();
    let is_nft = config.token_type == "Non-Fungible";
    let token_key = if is_nft { "non_fungible" } else { "fungible" };
    
    // Start contractimpl block
    code.push_str("#[contractimpl]\n");
    code.push_str(&format!("impl {} {{\n", config.name));
    
    // Constructor
    code.push_str("    pub fn __constructor(\n");
    code.push_str("        e: &Env");
    
    // Collect constructor arguments
    let mut constructor_args = Vec::new();
    
    // Access control arguments
    if config.access_control == "Ownable" {
        if let Some(args) = wizard_config["wizard"]["access_control"]["ownable"]["constructor_args"].as_array() {
            for arg in args {
                if let Some(arg_str) = arg.as_str() {
                    constructor_args.push(arg_str.to_string());
                }
            }
        }
    } else if config.access_control == "Roles" {
        if let Some(args) = wizard_config["wizard"]["access_control"]["roles"]["constructor_args"].as_array() {
            for arg in args {
                if let Some(arg_str) = arg.as_str() {
                    constructor_args.push(arg_str.to_string());
                }
            }
        }
    }
    
    // Add feature-specific constructor arguments
    for feature in &config.features {
        let arg_key = if config.access_control == "Ownable" {
            "constructor_args_ownable"
        } else if config.access_control == "Roles" {
            "constructor_args_roles"
        } else {
            "constructor_args"
        };
        
        if let Some(args) = wizard_config["wizard"]["features"][feature][arg_key].as_array() {
            for arg in args {
                if let Some(arg_str) = arg.as_str() {
                    if !constructor_args.contains(&arg_str.to_string()) {
                        constructor_args.push(arg_str.to_string());
                    }
                }
            }
        }
    }
    
    // Add limitation constructor arguments (manager role for Roles)
    if let Some(ref lim) = config.limitation {
        if config.access_control == "Roles" {
            if let Some(args) = wizard_config["wizard"]["limitations"][lim.as_str()]["constructor_args_roles"].as_array() {
                for arg in args {
                    if let Some(arg_str) = arg.as_str() {
                        if !constructor_args.contains(&arg_str.to_string()) {
                            constructor_args.push(arg_str.to_string());
                        }
                    }
                }
            }
        }
    }
    
    // If NFT minting is enabled with Roles, add minter constructor arg
    if config.nft_minting.is_some() && config.access_control == "Roles" {
        let arg = "minter: Address".to_string();
        if !constructor_args.contains(&arg) {
            constructor_args.push(arg);
        }
    }
    
    // If premint is set and there's no access control, we need a recipient address
    if config.premint.is_some() && config.access_control == "None" {
        let arg = "initial_holder: Address".to_string();
        if !constructor_args.contains(&arg) {
            constructor_args.push(arg);
        }
    }
    
    for arg in &constructor_args {
        code.push_str(",\n        ");
        code.push_str(arg);
    }
    
    code.push_str("\n    ) {\n");
    
    // Constructor body - token metadata initialization
    if let Some(constructor_lines) = wizard_config["wizard"]["settings"][token_key]["constructor"].as_array() {
        for line in constructor_lines {
            if let Some(mut line_str) = line.as_str().map(|s| s.to_string()) {
                line_str = line_str.replace("\"<SYMBOL>\"", &format!("\"{}\"", config.symbol));
                if let Some(ref uri) = config.uri {
                    line_str = line_str.replace("\"<URI>\"", &format!("\"{}\"", uri));
                }
                code.push_str(&line_str);
                code.push('\n');
            }
        }
    }
    
    // Access control initialization
    if config.access_control == "Ownable" {
        if let Some(init) = wizard_config["wizard"]["access_control"]["ownable"]["constructor"].as_array() {
            for line in init {
                if let Some(line_str) = line.as_str() {
                    code.push_str(line_str);
                    code.push('\n');
                }
            }
        }
    } else if config.access_control == "Roles" {
        if let Some(init) = wizard_config["wizard"]["access_control"]["roles"]["constructor"].as_array() {
            for line in init {
                if let Some(line_str) = line.as_str() {
                    code.push_str(line_str);
                    code.push('\n');
                }
            }
        }
    }
    
    // Feature-specific constructor code
    for feature in &config.features {
        let init_key = if config.access_control == "Ownable" {
            "constructor_ownable"
        } else if config.access_control == "Roles" {
            "constructor_roles"
        } else {
            "constructor"
        };
        
        if let Some(init) = wizard_config["wizard"]["features"][feature][init_key].as_array() {
            for line in init {
                if let Some(line_str) = line.as_str() {
                    code.push_str(line_str);
                    code.push('\n');
                }
            }
        }
    }
    
    // Limitation: grant manager role in constructor if Roles
    if let Some(ref lim) = config.limitation {
        if config.access_control == "Roles" {
            if let Some(init) = wizard_config["wizard"]["limitations"][lim.as_str()]["constructor_roles"].as_array() {
                for line in init {
                    if let Some(line_str) = line.as_str() {
                        code.push_str(line_str);
                        code.push('\n');
                    }
                }
            }
        }
    }
    
    // NFT minting: grant minter role in constructor if Roles
    if config.nft_minting.is_some() && config.access_control == "Roles" {
        code.push_str("        access_control::grant_role_no_auth(e, &admin, &minter, &Symbol::new(e, \"minter\"));\n");
    }
    
    // Premint: mint initial supply in the constructor (fungible only)
    if let Some(ref premint_amount) = config.premint {
        let mint_target = if config.access_control == "Ownable" {
            "&owner"
        } else if config.access_control == "Roles" {
            "&admin"
        } else {
            "&initial_holder"
        };
        code.push_str(&format!(
            "        Base::mint(e, {}, {} * 10i128.pow(18));\n",
            mint_target, premint_amount
        ));
    }
    
    code.push_str("    }\n");
    
    // === FEATURE FUNCTIONS (in same impl block) ===
    for feature in &config.features {
        // Determine which implementation key to use
        let is_pausable = config.features.contains(&"pausable".to_string());
        let base_impl_key = if is_nft && feature == "mintable" {
            // Check NFT variant for mintable
            let variant = config.nft_variant.as_ref().map(|s| s.as_str()).unwrap_or("Base");
            match variant {
                "Enumerable" => "implementation_nft_enumerable",
                "Consecutive" => "implementation_nft_consecutive",
                _ => "implementation_nft",
            }
        } else {
            "implementation"
        };
        
        // When pausable is selected, try the _pausable variant first, then fall back
        let pausable_impl_key = format!("{base_impl_key}_pausable");
        let impl_array = if is_pausable {
            wizard_config["wizard"]["features"][feature][pausable_impl_key.as_str()]
                .as_array()
                .or_else(|| wizard_config["wizard"]["features"][feature][base_impl_key].as_array())
                .or_else(|| wizard_config["wizard"]["features"][feature]["implementation"].as_array())
        } else {
            wizard_config["wizard"]["features"][feature][base_impl_key]
                .as_array()
                .or_else(|| wizard_config["wizard"]["features"][feature]["implementation"].as_array())
        };
        
        if let Some(impl_lines) = impl_array {
            if !impl_lines.is_empty() {
                code.push_str("\n");
                
                for line in impl_lines {
                    if let Some(mut line_str) = line.as_str().map(|s| s.to_string()) {
                        // Replace role macros
                        line_str = replace_role_macro(&line_str, &config.access_control, feature);
                        code.push_str(&line_str);
                        code.push('\n');
                    }
                }
            }
        }
    }
    
    // NFT minting function (sequential or non-sequential)
    if let Some(ref mint_type) = config.nft_minting {
        let is_pausable = config.features.contains(&"pausable".to_string());
        let variant_type = config.nft_variant.as_ref().map(|s| s.as_str()).unwrap_or("Base");
        let type_name = if variant_type == "Enumerable" { "Enumerable" } else { "Base" };
        
        code.push_str("\n");
        if is_pausable {
            code.push_str("    #[when_not_paused]\n");
        }
        let role_line = replace_role_macro("    #[<ROLE>]", &config.access_control, "mintable");
        code.push_str(&role_line);
        code.push('\n');
        
        match mint_type.as_str() {
            "sequential" => {
                code.push_str("    pub fn mint(e: &Env, to: Address) -> u32 {\n");
                code.push_str(&format!("        {type_name}::sequential_mint(e, &to)\n"));
                code.push_str("    }\n");
            }
            "non_sequential" => {
                code.push_str("    pub fn mint(e: &Env, to: Address, token_id: u32) {\n");
                code.push_str(&format!("        {type_name}::non_sequential_mint(e, &to, token_id);\n"));
                code.push_str("    }\n");
            }
            _ => {}
        }
    }
    
    // Close contractimpl block
    code.push_str("}\n");
    
    Ok(code)
}

// Generate token trait implementation (FungibleToken or NonFungibleToken)
fn generate_token_trait_implementation(wizard_config: &Value, config: &ContractConfig) -> Result<String, Error> {
    let mut code = String::new();
    
    let is_nft = config.token_type == "Non-Fungible";
    let token_key = if is_nft { "non_fungible" } else { "fungible" };
    
    // Determine which settings_implementation to use
    let impl_key = if is_nft {
        let variant = config.nft_variant.as_ref().map(|s| s.as_str()).unwrap_or("Base");
        match variant {
            "Enumerable" => "settings_implementation_enumerable",
            "Consecutive" => "settings_implementation_consecutive",
            _ => "settings_implementation",
        }
    } else {
        "settings_implementation"
    };
    
    // Base implementation
    if let Some(impl_lines) = wizard_config["wizard"]["settings"][token_key][impl_key].as_array() {
        for line in impl_lines {
            if let Some(line_str) = line.as_str() {
                code.push_str(line_str);
                code.push('\n');
            }
        }
    }
    
    // Override ContractType for stablecoin limitation
    if let Some(ref lim) = config.limitation {
        match lim.as_str() {
            "allowlist" => {
                code = code.replace("type ContractType = Base;", "type ContractType = AllowList;");
            }
            "blocklist" => {
                code = code.replace("type ContractType = Base;", "type ContractType = BlockList;");
            }
            _ => {}
        }
    }
    
    // Add pausable overrides if pausable feature is selected
    if config.features.contains(&"pausable".to_string()) {
        let pausable_key = if is_nft {
            "settings_implementation_nft"
        } else {
            "settings_implementation"
        };
        
        if let Some(pausable_impl) = wizard_config["wizard"]["features"]["pausable"][pausable_key].as_array() {
            // Remove the closing brace temporarily
            if code.ends_with("}\n") {
                code.truncate(code.len() - 2);
            }
            
            for line in pausable_impl {
                if let Some(line_str) = line.as_str() {
                    code.push_str(line_str);
                    code.push('\n');
                }
            }
            
            code.push_str("}\n");
        }
    }
    
    Ok(code)
}

// Generate feature trait extensions (like FungibleBurnable, NonFungibleRoyalties)
fn generate_feature_extensions(wizard_config: &Value, config: &ContractConfig) -> Result<String, Error> {
    let mut code = String::new();
    let is_nft = config.token_type == "Non-Fungible";
    
    // Limitation trait extension (FungibleAllowList or FungibleBlockList)
    if let Some(ref lim) = config.limitation {
        if let Some(ext_lines) = wizard_config["wizard"]["limitations"][lim.as_str()]["extensions"].as_array() {
            for line in ext_lines {
                if let Some(line_str) = line.as_str() {
                    let mut processed = line_str.replace("<NAME>", &config.name);
                    processed = replace_role_macro(&processed, &config.access_control, &format!("limitation_{}", lim));
                    code.push_str(&processed);
                    code.push('\n');
                }
            }
            code.push('\n');
        }
    }
    
    for feature in &config.features {
        // Add extensions
        let is_pausable = config.features.contains(&"pausable".to_string());
        let base_ext_key = if is_nft && (feature == "burnable") {
            "extensions_nft"
        } else {
            "extensions"
        };
        
        // When pausable is selected, try the _pausable variant first, then fall back
        let pausable_ext_key = format!("{base_ext_key}_pausable");
        let ext_lines = if is_pausable {
            wizard_config["wizard"]["features"][feature][pausable_ext_key.as_str()]
                .as_array()
                .or_else(|| wizard_config["wizard"]["features"][feature][base_ext_key].as_array())
        } else {
            wizard_config["wizard"]["features"][feature][base_ext_key].as_array()
        };
        
        // Fall back to base "extensions" if the specific key was not found
        let ext_lines = ext_lines
            .or_else(|| wizard_config["wizard"]["features"][feature]["extensions"].as_array());
        
        if let Some(ext_lines) = ext_lines {
            for line in ext_lines {
                if let Some(line_str) = line.as_str() {
                    code.push_str(line_str);
                    code.push('\n');
                }
            }
            code.push('\n');
        }
    }
    
    Ok(code)
}

// Generate utility functions
fn generate_utils(wizard_config: &Value, config: &ContractConfig) -> Result<String, Error> {
    let mut code = String::new();
    
    // Access control utils
    if config.access_control == "Ownable" {
        if let Some(utils) = wizard_config["wizard"]["access_control"]["ownable"]["utils"].as_array() {
            for line in utils {
                if let Some(line_str) = line.as_str() {
                    code.push_str(line_str);
                    code.push('\n');
                }
            }
            code.push('\n');
        }
    } else if config.access_control == "Roles" {
        if let Some(utils) = wizard_config["wizard"]["access_control"]["roles"]["utils"].as_array() {
            for line in utils {
                if let Some(line_str) = line.as_str() {
                    code.push_str(line_str);
                    code.push('\n');
                }
            }
            code.push('\n');
        }
    }
    
    // Feature utils
    for feature in &config.features {
        // Check for role-specific utils (like upgradeable)
        let utils_key = if config.access_control == "Roles" && feature == "upgradeable" {
            "utils_roles"
        } else {
            "utils"
        };
        
        if let Some(utils) = wizard_config["wizard"]["features"][feature][utils_key].as_array() {
            for line in utils {
                if let Some(line_str) = line.as_str() {
                    let processed = replace_role_macro(line_str, &config.access_control, feature);
                    code.push_str(&processed);
                    code.push('\n');
                }
            }
            code.push('\n');
        } else if let Some(utils) = wizard_config["wizard"]["features"][feature]["utils"].as_array() {
            for line in utils {
                if let Some(line_str) = line.as_str() {
                    let processed = replace_role_macro(line_str, &config.access_control, feature);
                    code.push_str(&processed);
                    code.push('\n');
                }
            }
            code.push('\n');
        }
    }
    
    Ok(code)
}

// Helper to replace role macros
fn replace_role_macro(line: &str, access_control: &str, feature: &str) -> String {
    if access_control == "Ownable" {
        line.replace("#[<ROLE>]", "#[only_owner]")
    } else if access_control == "Roles" {
        match feature {
            "mintable" | "consecutive" | "enumerable" => line.replace("#[<ROLE>]", "#[only_role(minter)]"),
            "pausable" => line.replace("#[<ROLE>]", "#[only_role(pauser)]"),
            "upgradeable" => line.replace("#[<ROLE>]", "#[only_role(upgrader)]"),
            "limitation_allowlist" | "limitation_blocklist" => line.replace("#[<ROLE>]", "#[only_role(manager)]"),
            _ => line.to_string(),
        }
    } else {
        line.to_string()
    }
}

// Save the generated contract to a file
fn save_contract(path: &str, code: &str) -> Result<(), Error> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Error::ContractCreationFailed(format!("Failed to create directory: {}", e)))?;
    }
    
    // Write the contract code
    fs::write(path, code)
        .map_err(|e| Error::ContractCreationFailed(format!("Failed to write contract: {}", e)))?;
    
    Ok(())
}


fn open_wizard(global_args: &global::Args) -> Result<(), Error> {
    let printer = Print::new(global_args.quiet);

    printer.infoln("Opening OpenZeppelin Contract Wizard...");

    let url = "https://wizard.openzeppelin.com/stellar";

    webbrowser::open(url)
        .map_err(|e| Error::BrowserFailed(format!("Failed to open browser: {e}")))?;

    printer.checkln("Opened Contract Wizard in your default browser");
    printer.println("\nInstructions:");
    printer.println("   1. Configure your contract in the wizard");
    printer.println("   2. Click 'Download' to get your contract files");
    printer.println("   3. Extract the downloaded ZIP file");
    printer.println("   4. Move the contract folder to your contracts/ directory");
    printer.println("   5. Add the contract to your workspace Cargo.toml if needed");
    printer.println(
        "   6. You may need to modify your environments.toml file to add constructor arguments",
    );
    printer.infoln(
        "The wizard will generate a complete Soroban contract with your selected features!",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};

    fn create_test_cmd(from: Option<String>, ls: bool, from_wizard: bool) -> Cmd {
        Cmd {
            from,
            ls,
            from_wizard,
            from_cli,
            output: None,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_ls_command() {
        let cmd = create_test_cmd(None, true, false);
        let global_args = global::Args::default();

        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/contents/examples",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"name": "example1", "type": "dir"}, {"name": "example2", "type": "dir"}]"#)
        .create();

        let result = cmd.run(&global_args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_latest_release() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "tag_name": "v1.2.3",
                "name": "Release v1.2.3",
                "published_at": "2024-01-15T10:30:00Z"
            }"#,
        )
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/releases/latest",
            server_url()
        );
        let result = Cmd::fetch_latest_release_from_url(&mock_url).await;

        assert!(result.is_ok());
        let release = result.unwrap();
        assert_eq!(release.tag_name, "v1.2.3");
    }

    #[tokio::test]
    async fn test_fetch_latest_release_error() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/releases/latest",
            server_url()
        );
        let result = Cmd::fetch_latest_release_from_url(&mock_url).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_action_specified() {
        let cmd = create_test_cmd(None, false, false);
        let global_args = global::Args::default();
        let result = cmd.run(&global_args).await;
        assert!(matches!(result, Err(Error::NoActionSpecified)));
    }
}
