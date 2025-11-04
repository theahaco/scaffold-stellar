use clap::{Args, Parser};
use degit_rs::degit;
use std::fs::{copy, metadata, read_dir, remove_dir_all};
use std::path::PathBuf;
use std::process::Command;
use std::{env, io};

use super::{build, generate};
use stellar_cli::{commands::global, print::Print};

pub const FRONTEND_TEMPLATE: &str = "theahaco/scaffold-stellar-frontend";
const TUTORIAL_BRANCH: &str = "tutorial";

/// A command to initialize a new project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the project must be provided
    pub project_path: PathBuf,

    #[command(flatten)]
    vers: Vers,
}

#[derive(Args, Debug, Clone)]
#[group(multiple = false)]
struct Vers {
    /// Initialize the tutorial project instead of the default project
    #[arg(long, default_value_t = false)]
    pub tutorial: bool,

    /// Optional argument to specify a tagged version
    #[arg(long)]
    pub tag: Option<String>,
}

/// Errors that can occur during initialization
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to clone template: {0}")]
    DegitError(String),
    #[error("Project path contains invalid UTF-8 characters and cannot be converted to a string")]
    InvalidProjectPathEncoding,
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error(transparent)]
    GenerateError(#[from] generate::contract::Error),
}

impl Cmd {
    /// Run the initialization command
    ///
    /// # Example:
    ///
    /// ```
    /// /// From the command line
    /// stellar scaffold init /path/to/project
    /// ```
    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer: Print = Print::new(global_args.quiet);

        // Convert to absolute path to avoid issues when changing directories
        let absolute_project_path = self.project_path.canonicalize().unwrap_or_else(|_| {
            // If canonicalize fails (path doesn't exist yet), manually create absolute path
            if self.project_path.is_absolute() {
                self.project_path.clone()
            } else {
                env::current_dir()
                    .unwrap_or_default()
                    .join(&self.project_path)
            }
        });

        printer.infoln(format!(
            "Creating new Stellar project in {}",
            absolute_project_path.display()
        ));

        let project_str = absolute_project_path
            .to_str()
            .ok_or(Error::InvalidProjectPathEncoding)?;

        let mut repo = FRONTEND_TEMPLATE.to_string();
        if let Some(tag) = self.vers.tag.as_deref() {
            repo = format!("{repo}#{tag}");
        } else if self.vers.tutorial {
            repo = format!("{repo}#{TUTORIAL_BRANCH}");
        }
        degit(repo.as_str(), project_str);

        if metadata(&absolute_project_path).is_err()
            || read_dir(&absolute_project_path)?.next().is_none()
        {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {project_str}: directory is empty or missing",
            )));
        }

        // Copy .env.example to .env
        let example_path = absolute_project_path.join(".env.example");
        let env_path = absolute_project_path.join(".env");
        copy(example_path, env_path)?;

        // If git is installed, run init and make initial commit
        if git_exists() {
            git_init(&absolute_project_path);
            git_add(&absolute_project_path, &["-A"]);
            git_commit(&absolute_project_path, "initial commit");
        }

        // Update the project's OpenZeppelin examples with the latest editions
        if !self.vers.tutorial {
            let example_contracts = ["nft-enumerable", "fungible-allowlist"];

            for contract in example_contracts {
                self.update_oz_example(&absolute_project_path, contract, global_args)
                    .await?;
            }
        }

        // Install npm dependencies
        printer.infoln("Installing npm dependencies...");
        let npm_install_command = Command::new("npm")
            .arg("install")
            .current_dir(&absolute_project_path)
            .output()?;
        if !npm_install_command.status.success() {
            printer.warnln(
                "Failed to install dependencies, run 'npm install' in the project directory",
            );
        }

        // Build contracts and create contract clients
        printer.infoln("Building contracts and generating client code...");
        // Use clap to parse build command with defaults, then configure programmatically
        let mut build_command = build::Command::parse_from(["build", "--build-clients"]);
        build_command.build.manifest_path = Some(absolute_project_path.join("Cargo.toml"));
        build_command.build_clients_args.env = Some(build::clients::ScaffoldEnv::Development);
        build_command.build_clients_args.workspace_root = Some(absolute_project_path.clone());
        let mut build_args = global_args.clone();
        if !(global_args.verbose && global_args.very_verbose) {
            build_args.quiet = true;
        }

        if let Err(e) = build_command.run(&build_args).await {
            printer.warnln(format!("Failed to build contract clients: {e}"));
        }

        printer.blankln("\n\n");
        printer.checkln(format!("Project successfully created at {project_str}"));
        printer.blankln(" You can now run the application with:\n");
        printer.blankln(format!("\tcd {}", self.project_path.display()));
        if !npm_install_command.status.success() {
            printer.blankln("\tnpm install");
        }
        printer.blankln("\tnpm start\n");
        printer.blankln(" Happy hacking! 🚀");
        Ok(())
    }

    /// Updates the project with an Open Zeppelin example contract
    ///
    /// This method attempts to generate a contract from Open Zeppelin
    /// and prints a warning if it can't be found or generated.
    async fn update_oz_example(
        &self,
        absolute_project_path: &PathBuf,
        contract_path: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        let original_dir = env::current_dir()?;
        env::set_current_dir(absolute_project_path)?;

        let contracts_path = absolute_project_path.join("contracts");
        let existing_contract_path = contracts_path.join(contract_path);

        if existing_contract_path.exists() {
            remove_dir_all(&existing_contract_path)?;
        }

        let mut quiet_global_args = global_args.clone();
        quiet_global_args.quiet = true;

        let result = generate::contract::Cmd {
            from: Some(contract_path.to_owned()),
            ls: false,
            from_wizard: false,
            output: Some(
                contracts_path
                    .join(contract_path)
                    .to_string_lossy()
                    .into_owned(),
            ),
        }
        .run(&quiet_global_args)
        .await;

        // Restore directory before handling result
        let _ = env::set_current_dir(original_dir);

        match result {
            Ok(()) => {
                printer.infoln(format!(
                    "Successfully added OpenZeppelin example contract: {contract_path}"
                ));
            }
            Err(generate::contract::Error::ExampleNotFound(_)) => {
                printer.infoln(format!(
                    "Skipped missing OpenZeppelin example contract: {contract_path}"
                ));
            }
            Err(e) => {
                printer.warnln(format!(
                    "Failed to generate example contract: {contract_path}\n{e}"
                ));
            }
        }

        Ok(())
    }
}

// Check if git is installed and exists in PATH
fn git_exists() -> bool {
    Command::new("git").arg("--version").output().is_err()
}

// Initialize a new git repository
fn git_init(path: &PathBuf) {
    let _ = Command::new("git").arg("init").current_dir(path).output();
}

// Stage files for commit
fn git_add(path: &PathBuf, rest: &[&str]) {
    let mut args = vec!["add"];
    args.extend_from_slice(rest);
    let _ = Command::new("git").args(args).current_dir(path).output();
}

// Commit with message
fn git_commit(path: &PathBuf, message: &str) {
    let _ = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .output();
}
