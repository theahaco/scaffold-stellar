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
        let printer = Print::new(global_args.quiet);

        printer.infoln(format!(
            "Creating new Stellar project in {:?}",
            self.project_path
        ));

        let project_str = self
            .project_path
            .to_str()
            .ok_or(Error::InvalidProjectPathEncoding)?;
        degit(FRONTEND_TEMPLATE, project_str);

        if metadata(&self.project_path).is_err() || read_dir(&self.project_path)?.next().is_none() {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {project_str}: directory is empty or missing",
            )));
        }

        self.update_fungible_token_example(global_args).await?;

        printer.checkln(format!("Project successfully created at {project_str}"));
        Ok(())
    }

    async fn update_fungible_token_example(&self, global_args: &global::Args) -> Result<(), Error> {
        let original_dir = env::current_dir()?;
        env::set_current_dir(&self.project_path)?;

        let contracts_path = self.project_path.join("contracts");
        let fungible_token_path = contracts_path.join("fungible-token-interface");

        if fungible_token_path.exists() {
            remove_dir_all(&fungible_token_path)?;
        }

        let mut quiet_global_args = global_args.clone();
        quiet_global_args.quiet = true;

        generate::contract::Cmd {
            from: Some("fungible-token-interface".to_owned()),
            ls: false,
            from_wizard: false,
            output: Some(
                contracts_path
                    .join("fungible-token-interface")
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
