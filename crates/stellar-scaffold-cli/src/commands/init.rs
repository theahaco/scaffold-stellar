use clap::Parser;
use degit::degit;
use std::fs::{metadata, read_dir};
use std::io;

const FRONTEND_TEMPLATE: &str = "https://github.com/AhaLabs/scaffold-stellar-frontend";

/// A command to initialize a new project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the project must be provided
    pub project_path: String,
}

/// Errors that can occur during initialization
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to clone template: {0}")]
    DegitError(String),
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

impl Cmd {
    /// Run the initialization command
    ///
    /// # Example:
    ///
    /// ```
    /// /// From the command line
    /// stellar-scaffold init /path/to/project
    /// ```
    #[allow(clippy::unused_self)]
    pub fn run(&self) -> Result<(), Error> {
        eprintln!("ℹ️  Creating new Stellar project in {}", self.project_path);
        degit(FRONTEND_TEMPLATE, &self.project_path);

        // Verify that the project directory exists and is not empty
        if metadata(&self.project_path).is_err() || read_dir(&self.project_path)?.next().is_none() {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {}: directory is empty or missing",
                self.project_path
            )));
        }

        eprintln!("✅ Project successfully created at {}", self.project_path);
        Ok(())
    }
}
