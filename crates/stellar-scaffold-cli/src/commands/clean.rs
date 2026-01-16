use cargo_metadata::Metadata;
use clap::Parser;
use std::{fs, io, path::PathBuf, process::Command};
use stellar_cli::{commands::global, print::Print};
/// A command to clean the scaffold-generated artifacts from a project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Path to Cargo.toml
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] io::Error),
}

// cleans up scaffold artifacts
// - target/stellar ✅
// - packages/* (but not checked-into-git files like .gitkeep) ✅
// - src/contracts/* (but not checked-into-git files like util.ts) ✅
// - contract aliases (for local and test)
// - identity aliases (for local and test)

// - should this be deleting target/stellar/local and target/stellar/testnet specifically to avoid deleting mainnet?
// - what about target/packages?
// - what about target/wasm32v1-none/release/guess_the_number.wasm

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        printer.infoln("Starting workspace cleanup");

        let cargo_meta = match &self.manifest_path {
            Some(manifest_path) => cargo_metadata::MetadataCommand::new()
                .manifest_path(manifest_path)
                .no_deps()
                .exec()
                .unwrap(),
            _ => cargo_metadata::MetadataCommand::new()
                .no_deps()
                .exec()
                .unwrap(),
        };

        Self::clean_target_stellar(&cargo_meta, &printer)?;

        let workspace_root: PathBuf = cargo_meta.workspace_root.into();

        Self::clean_packages(&workspace_root, &printer)?;

        Self::clean_src_contracts(&workspace_root, &printer)?;

        Ok(())
    }

    fn clean_target_stellar(cargo_meta: &Metadata, printer: &Print) -> Result<(), Error> {
        let target_dir = &cargo_meta.target_directory;
        let stellar_dir = target_dir.join("stellar");
        if stellar_dir.exists() {
            fs::remove_dir_all(&stellar_dir)?; //todo handle unwrap
        } else {
            printer.infoln("Skipping target clean: {stellar_dir} does not exist");
        }
        Ok(())
    }

    fn clean_packages(workspace_root: &PathBuf, printer: &Print) -> Result<(), Error> {
        let packages_path: PathBuf = workspace_root.join("packages");
        let git_tracked_packages_entries =
            Self::git_tracked_entries(workspace_root.clone(), "packages");
        Self::clean_dir(
            workspace_root,
            &packages_path,
            git_tracked_packages_entries,
            printer,
        )
    }

    fn clean_src_contracts(workspace_root: &PathBuf, printer: &Print) -> Result<(), Error> {
        let src_contracts_path = workspace_root.join("src").join("contracts");
        let git_tracked_src_contract_entries: Vec<String> =
            Self::git_tracked_entries(workspace_root.clone(), "src/contracts");
        Self::clean_dir(
            workspace_root,
            &src_contracts_path,
            git_tracked_src_contract_entries,
            printer,
        )
    }

    // clean aliases
    //     if the .env file has XDG_CONFIG_HOME remove the file it specifies
    // otherwise look at the environments.toml file
    // i think that XDG_CONFIG_HOME is defaulting to .confg...
    // so, can we just always delete what is in XDG_CONFIG_HOME/stellar?

    // or should we really do the following?

    // for all development.accounts remove each with the stellar cli command stellar keys rm.
    // for all development.contracts remove each contract alias with the stellar cli command stellar contract alias remove

    fn git_tracked_entries(workspace_root: PathBuf, subdir: &str) -> Vec<String> {
        let output = Command::new("git")
            .args(["ls-files", subdir])
            .current_dir(workspace_root)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout
                    .lines()
                    .map(std::string::ToString::to_string)
                    .collect()
            }
            _ => {
                // If git command fails, return empty list (no files will be preserved)
                Vec::new()
            }
        }
    }

    // cleans the given directory while preserving git tracked files, as well as some common template files: utils.js and .gitkeep
    fn clean_dir(
        workspace_root: &PathBuf,
        dir_to_clean: &PathBuf,
        git_tracked_entries: Vec<String>,
        printer: &Print,
    ) -> Result<(), Error> {
        if dir_to_clean.exists() {
            for entry in fs::read_dir(dir_to_clean)? {
                let entry = entry?;
                let path = entry.path();
                let relative_path = path.strip_prefix(workspace_root).unwrap_or(&path);
                let relative_str = relative_path.to_string_lossy().replace('\\', "/");

                // Skip if this is a git-tracked file
                if git_tracked_entries.contains(&relative_str) {
                    continue;
                }

                // Preserve common template files regardless of git status
                let filename = path.file_name().and_then(|n| n.to_str());
                if let Some(name) = filename
                    && (name == "util.ts" || name == ".gitkeep")
                {
                    continue;
                }

                // Remove the file or directory
                if path.is_dir() {
                    fs::remove_dir_all(&path).unwrap();
                } else {
                    fs::remove_file(&path).unwrap();
                }
                printer.infoln(format!("Removed {relative_str}"));
            }
        } else {
            println!("Skipping clean: {} does not exist", dir_to_clean.display());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_workspace(temp_dir: &Path) -> PathBuf {
        let manifest_path = temp_dir.join("Cargo.toml");
        fs::write(
            &manifest_path,
            r#"[package]
name = "soroban-hello-world-contract"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]
"#,
        )
        .unwrap();

        let src_dir = temp_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "// dummy lib").unwrap();

        manifest_path
    }

    #[test]
    fn test_clean_target_stellar() {
        let global_args = global::Args::default();
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = create_test_workspace(&temp_dir.path());

        let target_stellar_path = temp_dir.path().join("target").join("stellar");
        std::fs::create_dir_all(&target_stellar_path).unwrap();

        let cmd = Cmd {
            manifest_path: Some(manifest_path),
        };
        assert!(cmd.run(&global_args).is_ok());

        assert!(
            !target_stellar_path.exists(),
            "target/stellar should be removed"
        );
    }

    #[test]
    fn test_clean_packages() {
        let global_args = global::Args::default();
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = create_test_workspace(&temp_dir.path());

        let packages_path = temp_dir.path().join("packages");
        let test_package_path = packages_path.join("test_contract_package");
        std::fs::create_dir_all(&test_package_path).unwrap();

        let gitkeep_path = packages_path.join(".gitkeep");
        fs::write(&gitkeep_path, "").unwrap();

        let cmd = Cmd {
            manifest_path: Some(manifest_path),
        };

        assert!(cmd.run(&global_args).is_ok());

        assert!(
            !test_package_path.exists(),
            "packages/test_contract_package/ should be removed"
        );
        assert!(
            gitkeep_path.exists(),
            "packages/.gitkeep should be preserved"
        );
    }

    #[test]
    fn test_clean_src_contracts() {
        let global_args = global::Args::default();
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = create_test_workspace(&temp_dir.path());

        let src_contracts_path = temp_dir.path().join("src").join("contracts");
        std::fs::create_dir_all(&src_contracts_path).unwrap();

        let test_contract_path = src_contracts_path.join("test_contract_client.js");
        fs::write(&test_contract_path, "").unwrap();

        let util_path = src_contracts_path.join("util.ts");
        fs::write(&util_path, "").unwrap();

        let cmd = Cmd {
            manifest_path: Some(manifest_path),
        };

        assert!(cmd.run(&global_args).is_ok());

        assert!(
            !test_contract_path.exists(),
            "src/contracts/test_contract_client.js should be removed"
        );
        assert!(
            util_path.exists(),
            "src/contracts/util.js should be preserved"
        );
    }
}
