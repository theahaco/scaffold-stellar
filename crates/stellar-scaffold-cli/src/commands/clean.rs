use clap::Parser;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use stellar_cli::commands::global;

/// Clean scaffold-generated artifacts
///
/// Removes generated files and artifacts from the scaffold build process:
/// - target/stellar directory
/// - packages/* (excluding git-tracked files like .gitkeep)
/// - src/contracts/* (excluding git-tracked files like util.ts)
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
}

impl Cmd {
    pub fn run(&self, _global_args: &global::Args) -> Result<(), Error> {
        let workspace_root = self.get_workspace_root()?;

        println!("🧹 Cleaning scaffold artifacts from {}", workspace_root.display());

        // Clean target/stellar
        Self::clean_target_stellar(&workspace_root)?;

        // Clean packages/* (keep git-tracked files)
        Self::clean_packages(&workspace_root)?;

        // Clean src/contracts/* (keep git-tracked files)
        Self::clean_src_contracts(&workspace_root)?;

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
            if let Some(name) = filename {
                if name == "util.ts" || name == ".gitkeep" {
                    continue;
                }
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
}
