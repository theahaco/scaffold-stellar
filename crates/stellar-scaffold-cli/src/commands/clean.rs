use clap::Parser;
use std::{fs, path::PathBuf, process::Command};
use stellar_cli::{commands::global, print::Print};
/// A command to clean the scaffold-generated artifacts from a project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {}

#[derive(thiserror::Error, Debug)]
pub enum Error {}

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

        let cargo_metadata = cargo_metadata::MetadataCommand::new()
            .no_deps()
            .exec()
            .unwrap();

        // clean target/stellar
        let target_dir = cargo_metadata.target_directory;
        let stellar_dir = target_dir.join("stellar");
        if stellar_dir.exists() {
            fs::remove_dir_all(&stellar_dir).unwrap(); //todo handle unwrap
        } else {
            println!("Skipping target clean: {stellar_dir} does not exist");
        }

        // clean packages/
        let workspace_root = cargo_metadata.workspace_root;
        let packages_path = workspace_root.join("packages");
        let src_contracts_path = workspace_root.join("src").join("contracts");

        let packages_git_tracked =
            self.git_tracked_entries(workspace_root.clone().into(), "packages");
        if packages_path.exists() {
            for entry in fs::read_dir(&packages_path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let relative_path = path.strip_prefix(&workspace_root).unwrap_or(&path);
                let relative_str = relative_path.as_os_str().to_str().unwrap().to_owned();

                // Skip if this is a git-tracked file
                if packages_git_tracked.contains(&relative_str) {
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
            println!("Skipping packages clean: {packages_path} does not exist");
        }

        // clean src/contracts
        let src_contracts_path = Path::new("src/contracts");
        let src_contract_git_tracked =
            self.git_tracked_entries(workspace_root.clone().into(), src_contracts_path); // fix this - is this compatible with windows ?
        if src_contracts_path.exists() {
            for entry in fs::read_dir(&src_contracts_path).unwrap() {
                let entry = entry.unwrap();
                println!("entry in src contracts path {:?}", entry);
                let path = entry.path();
                let relative_path = path.strip_prefix(&workspace_root).unwrap_or(&path);
                let relative_str = relative_path.as_os_str().to_str().unwrap().to_owned();

                // Skip if this is a git-tracked file
                if src_contract_git_tracked.contains(&relative_str) {
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
            println!("Skipping src/contracts clean: {src_contracts_path} does not exist");
        }

        Ok(())
    }

    // does it makes sense to rely on the `git` cli command? or would it be worth using git2 or gitoxide crates?
    fn git_tracked_entries(&self, workspace_root: PathBuf, subdir: PathBuf) -> Vec<String> {
        let output = Command::new("git")
            .args(["ls-files", subdir.as_os_str().to_str().unwrap()])
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
}
