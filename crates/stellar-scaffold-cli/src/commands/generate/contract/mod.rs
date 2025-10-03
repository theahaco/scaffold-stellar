use clap::Parser;
use flate2::read::GzDecoder;
use reqwest;
use serde::Deserialize;
use std::{fs, path::Path};
use stellar_cli::commands::global;
use stellar_cli::print::Print;
use tar::Archive;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

#[derive(Parser, Debug)]
pub struct Cmd {
    /// Clone contract from `OpenZeppelin` examples
    #[arg(long, conflicts_with_all = ["ls", "from_wizard"])]
    pub from: Option<String>,

    /// List available contract examples
    #[arg(long, conflicts_with_all = ["from", "from_wizard"])]
    pub ls: bool,

    /// Open contract generation wizard in browser
    #[arg(long, conflicts_with_all = ["from", "ls"])]
    pub from_wizard: bool,

    /// Output directory for the generated contract (defaults to contracts/<example-name>)
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    CargoToml(#[from] cargo_toml::Error),
    #[error(transparent)]
    TomlDeserialize(#[from] toml::de::Error),
    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Git command failed: {0}")]
    GitCloneFailed(String),
    #[error("Example '{0}' not found in OpenZeppelin stellar-contracts")]
    ExampleNotFound(String),
    #[error("Failed to open browser: {0}")]
    BrowserFailed(String),
    #[error("No action specified. Use --from, --ls, or --from-wizard")]
    NoActionSpecified,
}

impl Cmd {
    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        match (&self.from, self.ls, self.from_wizard) {
            (Some(example_name), _, _) => self.clone_example(example_name, global_args).await,
            (_, true, _) => self.list_examples(global_args).await,
            (_, _, true) => open_wizard(global_args),
            _ => Err(Error::NoActionSpecified),
        }
    }

    async fn clone_example(
        &self,
        example_name: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        printer.infoln(format!("Downloading example '{example_name}'..."));

        let dest_path = self
            .output
            .clone()
            .unwrap_or_else(|| format!("contracts/{example_name}"));

        let repo_cache_path = self.ensure_cache_updated().await?;

        // Check if the example exists
        let example_source_path = repo_cache_path.join(format!("examples/{example_name}"));
        if !example_source_path.exists() {
            return Err(Error::ExampleNotFound(example_name.to_string()));
        }

        // Create destination and copy example contents
        fs::create_dir_all(&dest_path)?;
        Self::copy_directory_contents(&example_source_path, Path::new(&dest_path))?;

        // Get the latest release tag we're using
        let Release { tag_name } = Self::fetch_latest_release().await?;

        // Read and update workspace Cargo.toml
        let workspace_cargo_path = Path::new("Cargo.toml");
        if workspace_cargo_path.exists() {
            Self::update_workspace_dependencies(
                workspace_cargo_path,
                &example_source_path,
                &tag_name,
                global_args,
            )?;
        } else {
            printer.warnln("Warning: No workspace Cargo.toml found in current directory.");
            printer
                .println("   You'll need to manually add required dependencies to your workspace.");
        }

        printer.checkln(format!(
            "Successfully downloaded example '{example_name}' to {dest_path}"
        ));
        printer
            .infoln("You may need to modify your environments.toml to add constructor arguments!");
        Ok(())
    }

    fn update_workspace_dependencies(
        workspace_path: &Path,
        example_path: &Path,
        tag: &str,
        global_args: &global::Args,
    ) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        let example_cargo_content = fs::read_to_string(example_path.join("Cargo.toml"))?;
        let deps = Self::extract_stellar_dependencies(&example_cargo_content)?;
        if deps.is_empty() {
            return Ok(());
        }

        // Parse the workspace Cargo.toml
        let mut manifest = cargo_toml::Manifest::from_path(workspace_path)?;

        // Ensure workspace.dependencies exists
        if manifest.workspace.is_none() {
            // Create a minimal workspace with just what we need
            let workspace_toml = r"
[workspace]
members = []

[workspace.dependencies]
";
            let workspace: cargo_toml::Workspace<toml::Value> = toml::from_str(workspace_toml)?;
            manifest.workspace = Some(workspace);
        }
        let workspace = manifest.workspace.as_mut().unwrap();

        let mut workspace_deps = workspace.dependencies.clone();

        let mut added_deps = Vec::new();
        let mut updated_deps = Vec::new();

        for dep in deps {
            let git_dep = cargo_toml::DependencyDetail {
                git: Some("https://github.com/OpenZeppelin/stellar-contracts".to_string()),
                tag: Some(tag.to_string()),
                ..Default::default()
            };

            if let Some(existing_dep) = workspace_deps.clone().get(&dep) {
                // Check if we need to update the tag
                if let cargo_toml::Dependency::Detailed(detail) = existing_dep
                    && let Some(existing_tag) = &detail.tag
                    && existing_tag != tag
                {
                    workspace_deps.insert(
                        dep.clone(),
                        cargo_toml::Dependency::Detailed(Box::new(git_dep)),
                    );
                    updated_deps.push((dep, existing_tag.clone()));
                }
            } else {
                workspace_deps.insert(
                    dep.clone(),
                    cargo_toml::Dependency::Detailed(Box::new(git_dep)),
                );
                added_deps.push(dep);
            }
        }

        if !added_deps.is_empty() || !updated_deps.is_empty() {
            workspace.dependencies = workspace_deps;
            // Write the updated manifest back to file
            let toml_string = toml::to_string_pretty(&manifest)?;
            fs::write(workspace_path, toml_string)?;

            if !added_deps.is_empty() {
                printer.infoln("Added the following dependencies to workspace:");
                for dep in added_deps {
                    printer.println(format!("   • {dep}"));
                }
            }

            if !updated_deps.is_empty() {
                printer.infoln("Updated the following dependencies:");
                for (dep, old_tag) in updated_deps {
                    printer.println(format!("   • {dep}: {old_tag} -> {tag}"));
                }
            }
        }

        Ok(())
    }

    fn extract_stellar_dependencies(cargo_toml_content: &str) -> Result<Vec<String>, Error> {
        let manifest: cargo_toml::Manifest = toml::from_str(cargo_toml_content)?;

        Ok(manifest
            .dependencies
            .iter()
            .filter(|(dep_name, _)| dep_name.starts_with("stellar-"))
            .filter_map(|(dep_name, dep_detail)| match dep_detail {
                cargo_toml::Dependency::Detailed(detail)
                    if !(detail.inherited || detail.git.is_some()) =>
                {
                    None
                }
                _ => Some(dep_name.clone()),
            })
            .collect())
    }

    async fn list_examples(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        printer.infoln("Fetching available contract examples...");

        let repo_cache_path = self.ensure_cache_updated().await?;
        let examples_path = repo_cache_path.join("examples");

        let mut examples: Vec<String> = if examples_path.exists() {
            fs::read_dir(examples_path)?
                .filter_map(std::result::Result::ok)
                .filter(|entry| entry.path().is_dir())
                .filter_map(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(std::string::ToString::to_string)
                })
                .collect()
        } else {
            Vec::new()
        };

        examples.sort();

        printer.println("\nAvailable contract examples:");
        printer.println("────────────────────────────────");

        for example in &examples {
            printer.println(format!("  📁 {example}"));
        }

        printer.println("\nUsage:");
        printer.println("   stellar-scaffold contract generate --from <example-name>");
        printer.println("   Example: stellar-scaffold contract generate --from nft-royalties");

        Ok(())
    }

    async fn fetch_latest_release() -> Result<Release, Error> {
        Self::fetch_latest_release_from_url(
            "https://api.github.com/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .await
    }

    async fn fetch_latest_release_from_url(url: &str) -> Result<Release, Error> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "stellar-scaffold-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
        }

        let release: Release = response.json().await?;
        Ok(release)
    }

    async fn cache_repository(repo_cache_path: &Path, tag_name: &str) -> Result<(), Error> {
        fs::create_dir_all(repo_cache_path)?;

        // Download and extract the specific tag directly
        Self::download_and_extract_tag(repo_cache_path, tag_name).await?;

        if repo_cache_path.read_dir()?.next().is_none() {
            return Err(Error::GitCloneFailed(format!(
                "Failed to download repository release {tag_name} to cache"
            )));
        }

        Ok(())
    }

    async fn download_and_extract_tag(dest_path: &Path, tag_name: &str) -> Result<(), Error> {
        let url =
            format!("https://github.com/OpenZeppelin/stellar-contracts/archive/{tag_name}.tar.gz",);

        // Download the tar.gz file
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", "stellar-scaffold-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::GitCloneFailed(format!(
                "Failed to download release {tag_name}: HTTP {}",
                response.status()
            )));
        }

        // Get the response bytes
        let bytes = response.bytes().await?;

        // Extract the tar.gz in a blocking task to avoid blocking the async runtime
        let dest_path = dest_path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let tar = GzDecoder::new(std::io::Cursor::new(bytes));
            let mut archive = Archive::new(tar);

            for entry in archive.entries()? {
                let mut entry = entry?;
                let path = entry.path()?;

                // Strip the root directory (stellar-contracts-{tag}/)
                let stripped_path = path.components().skip(1).collect::<std::path::PathBuf>();

                if stripped_path.as_os_str().is_empty() {
                    continue;
                }

                let dest_file_path = dest_path.join(&stripped_path);

                if entry.header().entry_type().is_dir() {
                    std::fs::create_dir_all(&dest_file_path)?;
                } else {
                    if let Some(parent) = dest_file_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    entry.unpack(&dest_file_path)?;
                }
            }

            Ok::<(), std::io::Error>(())
        })
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?
        .map_err(Error::Io)?;

        Ok(())
    }

    async fn ensure_cache_updated(&self) -> Result<std::path::PathBuf, Error> {
        let cache_dir = dirs::cache_dir().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache directory not found",
            ))
        })?;

        let base_cache_path = cache_dir.join("stellar-scaffold-cli/openzeppelin-stellar-contracts");

        // Get the latest release tag
        let Release { tag_name } = Self::fetch_latest_release().await?;
        let repo_cache_path = base_cache_path.join(&tag_name);
        if !repo_cache_path.exists() {
            Self::cache_repository(&repo_cache_path, &tag_name).await?;
        }

        Ok(repo_cache_path)
    }

    fn copy_directory_contents(source: &Path, dest: &Path) -> Result<(), Error> {
        let copy_options = fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true);

        fs_extra::dir::copy(source, dest, &copy_options)
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        Ok(())
    }
}

fn open_wizard(global_args: &global::Args) -> Result<(), Error> {
    let printer = Print::new(global_args.quiet);

    printer.infoln("Opening OpenZeppelin Contract Wizard...");

    let url = "https://wizard.openzeppelin.com/stellar";

    webbrowser::open(url)
        .map_err(|e| Error::BrowserFailed(format!("Failed to open browser: {e}")))?;

    printer.checkln("Opened Contract Wizard in your default browser");
    printer.println("\nInstructions:");
    printer.println("   1. Configure your contract in the wizard");
    printer.println("   2. Click 'Download' to get your contract files");
    printer.println("   3. Extract the downloaded ZIP file");
    printer.println("   4. Move the contract folder to your contracts/ directory");
    printer.println("   5. Add the contract to your workspace Cargo.toml if needed");
    printer.println(
        "   6. You may need to modify your environments.toml file to add constructor arguments",
    );
    printer.infoln(
        "The wizard will generate a complete Soroban contract with your selected features!",
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};

    fn create_test_cmd(from: Option<String>, ls: bool, from_wizard: bool) -> Cmd {
        Cmd {
            from,
            ls,
            from_wizard,
            output: None,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_ls_command() {
        let cmd = create_test_cmd(None, true, false);
        let global_args = global::Args::default();

        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/contents/examples",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"name": "example1", "type": "dir"}, {"name": "example2", "type": "dir"}]"#)
        .create();

        let result = cmd.run(&global_args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fetch_latest_release() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "tag_name": "v1.2.3",
                "name": "Release v1.2.3",
                "published_at": "2024-01-15T10:30:00Z"
            }"#,
        )
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/releases/latest",
            server_url()
        );
        let result = Cmd::fetch_latest_release_from_url(&mock_url).await;

        assert!(result.is_ok());
        let release = result.unwrap();
        assert_eq!(release.tag_name, "v1.2.3");
    }

    #[tokio::test]
    async fn test_fetch_latest_release_error() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/releases/latest",
        )
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/releases/latest",
            server_url()
        );
        let result = Cmd::fetch_latest_release_from_url(&mock_url).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_action_specified() {
        let cmd = create_test_cmd(None, false, false);
        let global_args = global::Args::default();
        let result = cmd.run(&global_args).await;
        assert!(matches!(result, Err(Error::NoActionSpecified)));
    }
}
