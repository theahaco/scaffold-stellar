use clap::Parser;
use rust_embed::{EmbeddedFile, RustEmbed};
use soroban_cli::commands::contract::init as soroban_init;
use std::{
    fs::{self, create_dir_all, metadata, write, Metadata},
    io,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;
use toml_edit::TomlError;

const FRONTEND_TEMPLATE: &str = "https://github.com/AhaLabs/scaffold-stellar-frontend";

#[derive(RustEmbed)]
#[folder = "./src/examples/soroban/status_message"]
struct ExampleStatusMessage;

/// A command to initialize a new project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the project must be provided to initialize
    pub project_path: PathBuf,
    /// The name of the project
    #[arg(default_value = "stellar-example")]
    pub name: String,
}
/// Errors that can occur during initialization
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Io error: {0}")]
    IoError(#[from] io::Error),
    #[error("Soroban init error: {0}")]
    SorobanInitError(#[from] soroban_init::Error),
    #[error("Failed to convert bytes to string: {0}")]
    ConverBytesToStringErr(#[from] std::str::Utf8Error),
    #[error("Failed to parse toml file: {0}")]
    TomlParseError(#[from] TomlError),
    #[error("Failed to copy frontend files: {0}")]
    FrontendCopyError(String),
    #[error("Git clone failed: {0}")]
    GitCloneError(String),
}

impl Cmd {
    /// Run the initialization command by calling the soroban init command
    ///
    /// # Example:
    ///
    /// ```
    /// /// From the command line
    /// stellar-scaffold init /path/to/project
    /// ```
    #[allow(clippy::unused_self)]
    pub fn run(&self) -> Result<(), Error> {
        // Create a new project using the soroban init command
        // by default uses a provided frontend template
        // Examples cannot currently be added by user
        soroban_init::Cmd {
            project_path: self.project_path.to_string_lossy().to_string(),
            name: self.name.clone(),
            with_example: None,
            overwrite: true,
            frontend_template: None,
        }
        .run(&soroban_cli::commands::global::Args::default())?;

        // Clone frontend template
        let fe_template_dir = tempfile::tempdir().map_err(|e| {
            eprintln!("Error creating temp dir for frontend template");
            Error::IoError(e)
        })?;

        clone_repo(FRONTEND_TEMPLATE, fe_template_dir.path())?;
        copy_frontend_files(&fe_template_dir, &self.project_path)?;

        copy_example_contracts(&self.project_path)?;
        rename_cargo_toml_remove(&self.project_path, "status_message")?;
        Ok(())
    }
}

fn copy_example_contracts(to: &Path) -> Result<(), Error> {
    for item in ExampleStatusMessage::iter() {
        copy_file(
            &to.join("contracts/status_message"),
            item.as_ref(),
            ExampleStatusMessage::get(&item),
        )?;
    }

    Ok(())
}

fn copy_file(
    example_path: &Path,
    filename: &str,
    embedded_file: Option<EmbeddedFile>,
) -> Result<(), Error> {
    let to = example_path.join(filename);
    if file_exists(&to) {
        println!(
            "ℹ️  Skipped creating {} as it already exists",
            &to.to_string_lossy()
        );
        return Ok(());
    }
    create_dir_all(to.parent().expect("invalid path")).map_err(|e| {
        eprintln!("Error creating directory path for: {to:?}");
        e
    })?;

    let Some(embedded_file) = embedded_file else {
        println!("⚠️  Failed to read file: {filename}");
        return Ok(());
    };

    let file_contents = std::str::from_utf8(embedded_file.data.as_ref()).map_err(|e| {
        eprintln!("Error converting file contents in {filename:?} to string",);
        e
    })?;

    println!("➕  Writing {}", &to.to_string_lossy());
    write(&to, file_contents).map_err(|e| {
        eprintln!("Error writing file: {to:?}");
        e
    })?;
    Ok(())
}

fn file_exists(file_path: &Path) -> bool {
    metadata(file_path)
        .as_ref()
        .map(Metadata::is_file)
        .unwrap_or(false)
}

fn rename_cargo_toml_remove(project: &Path, name: &str) -> Result<(), Error> {
    let from = project.join(format!("contracts/{name}/Cargo.toml.remove"));
    let to = from.with_extension("");
    println!("Renaming to {from:?} to {to:?}");
    fs::rename(from, to)?;
    Ok(())
}

fn clone_repo(repo_url: &str, dest: &Path) -> Result<(), Error> {
    let status = Command::new("git")
        .args(["clone", repo_url, dest.to_str().unwrap()])
        .status()
        .map_err(|e| Error::GitCloneError(format!("Failed to execute git clone: {e}")))?;

    if !status.success() {
        return Err(Error::GitCloneError("Git clone command failed".to_string()));
    }
    Ok(())
}

fn copy_frontend_files(temp_dir: &TempDir, project_path: &Path) -> Result<(), Error> {
    let entries = std::fs::read_dir(temp_dir.path())?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != ".git");

    for entry in entries {
        fs_extra::copy_items(
            &[entry.path()],
            project_path,
            &fs_extra::dir::CopyOptions::new()
                .overwrite(true)
                .skip_exist(false),
        )
        .map_err(|e| Error::FrontendCopyError(e.to_string()))?;
    }

    Ok(())
}
