use clap::Parser;
use std::{fs, io, path::PathBuf, process::Command};
use stellar_cli::{commands::global, print::Print};
/// A command to clean the scaffold-generated artifacts from a project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {}

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
        let workspace_root: PathBuf = cargo_metadata.workspace_root.into();

        self.clean_packages(&workspace_root, &printer)?;

        self.clean_src_contracts(&workspace_root, &printer)?;

        Ok(())
    }

    fn clean_packages(&self, workspace_root: &PathBuf, printer: &Print) -> Result<(), Error> {
        let packages_path: PathBuf = workspace_root.join("packages").into();
        let git_tracked_packages_entries =
            self.git_tracked_entries(workspace_root.clone().into(), "packages");
        self.clean_dir(
            &workspace_root,
            &packages_path,
            git_tracked_packages_entries,
            printer,
        )
    }

    fn clean_src_contracts(&self, workspace_root: &PathBuf, printer: &Print) -> Result<(), Error> {
        let src_contracts_path = workspace_root.join("src").join("contracts");
        let git_tracked_src_contract_entries: Vec<String> =
            self.git_tracked_entries(workspace_root.clone().into(), "src/contracts");
        self.clean_dir(
            &workspace_root,
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

    fn git_tracked_entries(&self, workspace_root: PathBuf, subdir: &str) -> Vec<String> {
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
        &self,
        workspace_root: &PathBuf,
        dir_to_clean: &PathBuf,
        git_tracked_entries: Vec<String>,
        printer: &Print,
    ) -> Result<(), Error> {
        if dir_to_clean.exists() {
            for entry in fs::read_dir(&dir_to_clean)? {
                let entry = entry?;
                let path = entry.path();
                let relative_path = path.strip_prefix(&workspace_root).unwrap_or(&path);
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
            println!("Skipping clean: {dir_to_clean:?} does not exist");
        }

        Ok(())
    }
}
