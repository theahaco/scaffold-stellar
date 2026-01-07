use clap::Parser;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use stellar_cli::commands::global;
use stellar_cli::config::locator;

use crate::commands::build::clients::ScaffoldEnv;
use crate::commands::build::env_toml::{self, Environment};

/// Clean scaffold-generated artifacts
///
/// Removes generated files and artifacts from the scaffold build process:
/// - target/stellar directory
/// - packages/* (excluding git-tracked files like .gitkeep)
/// - src/contracts/* (excluding git-tracked files like util.ts)
/// - contract aliases and created accounts from config
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Path to Cargo.toml
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to get current directory: {0}")]
    CurrentDir(io::Error),
    #[error("Failed to find workspace root from manifest path")]
    WorkspaceRoot,
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Metadata(#[from] cargo_metadata::Error),
    #[error(transparent)]
    EnvironmentsToml(#[from] env_toml::Error),
    #[error(transparent)]
    ConfigLocator(#[from] locator::Error),
}

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let workspace_root = self.get_workspace_root()?;

        println!("🧹 Cleaning scaffold artifacts from {}", workspace_root.display());

        // Clean target/stellar
        Self::clean_target_stellar(&workspace_root)?;

        // Clean packages/* (keep git-tracked files)
        Self::clean_packages(&workspace_root)?;

        // Clean src/contracts/* (keep git-tracked files)
        Self::clean_src_contracts(&workspace_root)?;

        // Clean config (accounts and contract aliases)
        Self::clean_config(&workspace_root, global_args)?;

        println!("✨ Clean complete!");

        Ok(())
    }

    fn get_workspace_root(&self) -> Result<PathBuf, Error> {
        let current_dir = std::env::current_dir().map_err(Error::CurrentDir)?;

        let manifest_path = if let Some(path) = &self.manifest_path {
            if path.is_absolute() {
                path.clone()
            } else {
                current_dir.join(path)
            }
        } else {
            current_dir.join("Cargo.toml")
        };

        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(&manifest_path)
            .exec()?;

        Ok(metadata.workspace_root.into_std_path_buf())
    }

    fn clean_target_stellar(workspace_root: &Path) -> Result<(), Error> {
        let target_stellar = workspace_root.join("target").join("stellar");

        if target_stellar.exists() {
            println!("  Removing target/stellar/");
            fs::remove_dir_all(&target_stellar)?;
        } else {
            println!("  target/stellar/ does not exist, skipping");
        }

        Ok(())
    }

    fn clean_packages(workspace_root: &Path) -> Result<(), Error> {
        let packages_dir = workspace_root.join("packages");

        if !packages_dir.exists() {
            println!("  packages/ does not exist, skipping");
            return Ok(());
        }

        // Get list of git-tracked files in packages/
        let git_tracked = Self::get_git_tracked_files(workspace_root, "packages");

        println!("  Cleaning packages/ (preserving git-tracked files)");

        // Iterate through packages/ directory
        for entry in fs::read_dir(&packages_dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative_path = path.strip_prefix(workspace_root).unwrap_or(&path);
            let relative_str = relative_path.to_string_lossy().replace('\\', "/");

            // Skip if this is a git-tracked file
            if git_tracked.contains(&relative_str) {
                continue;
            }

            // Also preserve .gitkeep files regardless of git status
            if path.file_name().and_then(|n| n.to_str()) == Some(".gitkeep") {
                continue;
            }

            // Remove the file or directory
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
            println!("    Removed {relative_str}");
        }

        Ok(())
    }

    fn clean_src_contracts(workspace_root: &Path) -> Result<(), Error> {
        let src_contracts_dir = workspace_root.join("src").join("contracts");

        if !src_contracts_dir.exists() {
            println!("  src/contracts/ does not exist, skipping");
            return Ok(());
        }

        // Get list of git-tracked files in src/contracts/
        let git_tracked = Self::get_git_tracked_files(workspace_root, "src/contracts");

        println!("  Cleaning src/contracts/ (preserving git-tracked files)");

        // Iterate through src/contracts/ directory
        for entry in fs::read_dir(&src_contracts_dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative_path = path.strip_prefix(workspace_root).unwrap_or(&path);
            let relative_str = relative_path.to_string_lossy().replace('\\', "/");

            // Skip if this is a git-tracked file
            if git_tracked.contains(&relative_str) {
                continue;
            }

            // Also preserve common template files regardless of git status
            let filename = path.file_name().and_then(|n| n.to_str());
            if let Some(name) = filename
                && (name == "util.ts" || name == ".gitkeep")
            {
                continue;
            }

            // Remove the file or directory
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
            println!("    Removed {relative_str}");
        }

        Ok(())
    }

    fn get_git_tracked_files(workspace_root: &Path, subdir: &str) -> Vec<String> {
        use std::process::Command;

        // Try to get git-tracked files, if git is not available or not a git repo, return empty list
        let output = Command::new("git")
            .args(["ls-files", subdir])
            .current_dir(workspace_root)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().map(std::string::ToString::to_string).collect()
            }
            _ => {
                // If git command fails, return empty list (no files will be preserved)
                Vec::new()
            }
        }
    }

    fn clean_config(workspace_root: &Path, _global_args: &global::Args) -> Result<(), Error> {
        // Check if .env file has XDG_CONFIG_HOME and remove the file it specifies
        let env_file = workspace_root.join(".env");
        if env_file.exists() {
            let env_content = fs::read_to_string(&env_file)?;
            for line in env_content.lines() {
                if line.starts_with("XDG_CONFIG_HOME=") {
                    let config_path = line.split('=').nth(1).map(|s| s.trim().trim_matches('"'));
                    if let Some(path_str) = config_path {
                        let config_dir = PathBuf::from(path_str);
                        if config_dir.exists() {
                            println!("  Removing config directory from XDG_CONFIG_HOME: {}", config_dir.display());
                            fs::remove_dir_all(&config_dir)?;
                        }
                    }
                    return Ok(());
                }
            }
        }

        // Otherwise look at environments.toml file
        let env_toml_path = workspace_root.join(env_toml::ENV_FILE);
        if !env_toml_path.exists() {
            println!("  No environments.toml found, skipping config cleanup");
            return Ok(());
        }

        // Try to get development environment
        match Environment::get(workspace_root, &ScaffoldEnv::Development) {
            Ok(Some(env)) => {
                println!("  Cleaning config (accounts and aliases)");

                // Remove accounts using stellar keys rm
                if let Some(accounts) = &env.accounts {
                    for account in accounts {
                        let result = std::process::Command::new("stellar")
                            .args(["keys", "rm", &account.name])
                            .output();

                        match result {
                            Ok(output) if output.status.success() => {
                                println!("    Removed account: {}", account.name);
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                if !stderr.contains("not found") {
                                    eprintln!("    Warning: Failed to remove account {}: {}", account.name, stderr);
                                }
                            }
                            Err(e) => {
                                eprintln!("    Warning: Failed to execute stellar keys rm: {e}");
                            }
                        }
                    }
                }

                // Remove contract aliases using stellar contract alias remove
                if let Some(contracts) = &env.contracts {
                    for (contract_name, _) in contracts {
                        let result = std::process::Command::new("stellar")
                            .args(["contract", "alias", "remove", contract_name])
                            .output();

                        match result {
                            Ok(output) if output.status.success() => {
                                println!("    Removed contract alias: {contract_name}");
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                if !stderr.contains("not found") && !stderr.contains("No alias") {
                                    eprintln!("    Warning: Failed to remove contract alias {contract_name}: {stderr}");
                                }
                            }
                            Err(e) => {
                                eprintln!("    Warning: Failed to execute stellar contract alias remove: {e}");
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                println!("  No development environment found in environments.toml");
            }
            Err(e) => {
                eprintln!("  Warning: Failed to read environments.toml: {e}");
            }
        }

        Ok(())
    }
}
