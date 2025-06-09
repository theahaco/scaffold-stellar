use clap::Parser;
use degit::degit;
use std::fs::{metadata, read_dir, remove_dir_all};
use std::io;
use std::path::PathBuf;

use super::generate;

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
    pub async fn run(&self) -> Result<(), Error> {
        eprintln!(
            "ℹ️  Creating new Stellar project in {:?}",
            self.project_path
        );
        let project_str = self
            .project_path
            .to_str()
            .ok_or(Error::InvalidProjectPathEncoding)?;
        degit(FRONTEND_TEMPLATE, project_str);

        if metadata(&self.project_path).is_err() || read_dir(&self.project_path)?.next().is_none() {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {:?}: directory is empty or missing",
                self.project_path
            )));
        }

        self.update_fungible_token_example().await?;

        eprintln!("✅ Project successfully created at {:?}", self.project_path);
        Ok(())
    }

    async fn update_fungible_token_example(&self) -> Result<(), Error> {
        let contracts_path = self.project_path.join("contracts");
        let fungible_token_path = contracts_path.join("fungible-token-interface");

        if fungible_token_path.exists() {
            remove_dir_all(&fungible_token_path)?;
        }

        generate::contract::Cmd {
            from: Some("fungible-token-interface".to_owned()),
            ls: false,
            from_wizard: false,
            output: Some(contracts_path.to_string_lossy().into_owned()),
        }
        .run()
        .await?;
        Ok(())
    }
}
