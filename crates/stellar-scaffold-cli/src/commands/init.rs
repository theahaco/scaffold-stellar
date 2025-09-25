use clap::Parser;
use degit::degit;
use std::fs::{metadata, read_dir, remove_dir_all};
use std::path::PathBuf;
use std::{env, io};

use super::generate;
use stellar_cli::{commands::global, print::Print};

const FRONTEND_TEMPLATE: &str = "https://github.com/AhaLabs/scaffold-stellar-frontend";

/// A command to initialize a new project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the project must be provided
    pub project_path: PathBuf,
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
            "Creating new Stellar project in {}", absolute_project_path.display()
        ));

        let project_str = absolute_project_path
            .to_str()
            .ok_or(Error::InvalidProjectPathEncoding)?;

        degit(FRONTEND_TEMPLATE, project_str);

        if metadata(&absolute_project_path).is_err()
            || read_dir(&absolute_project_path)?.next().is_none()
        {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {project_str}: directory is empty or missing",
            )));
        }

        // Update the project with the latest OpenZeppelin examples
        self.update_oz_example(
            &absolute_project_path,
            "fungible-token-interface",
            global_args,
        )
        .await?;
        self.update_oz_example(&absolute_project_path, "nft-enumerable", global_args)
            .await?;

        printer.checkln(format!("Project successfully created at {project_str}"));
        Ok(())
    }

    async fn update_oz_example(
        &self,
        absolute_project_path: &PathBuf,
        contract_path: &'static str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let original_dir = env::current_dir()?;
        env::set_current_dir(absolute_project_path)?;

        let contracts_path = absolute_project_path.join("contracts");
        let existing_contract_path = contracts_path.join(contract_path);

        if existing_contract_path.exists() {
            remove_dir_all(&existing_contract_path)?;
        }

        let mut quiet_global_args = global_args.clone();
        quiet_global_args.quiet = true;

        generate::contract::Cmd {
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
        .await?;
        env::set_current_dir(original_dir)?;
        Ok(())
    }
}
