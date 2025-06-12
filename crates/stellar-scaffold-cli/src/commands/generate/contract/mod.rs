use clap::Parser;
use reqwest;
use serde::Deserialize;
use std::{fs, path::Path};

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
    pub async fn run(&self) -> Result<(), Error> {
        match (&self.from, self.ls, self.from_wizard) {
            (Some(example_name), _, _) => self.clone_example(example_name).await,
            (_, true, _) => self.list_examples().await,
            (_, _, true) => open_wizard(),
            _ => Err(Error::NoActionSpecified),
        }
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
        let cache_ref_file = repo_cache_path.join(".release_ref");

        let should_update_cache = if repo_cache_path.exists() {
            if let Ok(cached_tag) = fs::read_to_string(&cache_ref_file) {
                if cached_tag.trim() == tag_name {
                    eprintln!("📂 Using cached repository (release {tag_name})...");
                    false
                } else {
                    eprintln!("📂 New release available ({tag_name}). Updating cache...");
                    true
                }
            } else {
                eprintln!("📂 Cache metadata missing. Updating...");
                true
            }
        } else {
            eprintln!("📂 Cache not found. Downloading release {tag_name}...");
            true
        };

        if should_update_cache {
            if repo_cache_path.exists() {
                fs::remove_dir_all(&repo_cache_path)?;
            }
            Self::cache_repository(&repo_cache_path, &cache_ref_file, &tag_name)?;
        }

        Ok(repo_cache_path)
    }

    async fn clone_example(&self, example_name: &str) -> Result<(), Error> {
        eprintln!("🔍 Downloading example '{example_name}'...");

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

        // Create destination and copy contents
        fs::create_dir_all(&dest_path)?;
        Self::copy_directory_contents(&example_source_path, Path::new(&dest_path))?;

        eprintln!("✅ Successfully downloaded example '{example_name}' to {dest_path}");
        Ok(())
    }

    async fn list_examples(&self) -> Result<(), Error> {
        eprintln!("📋 Fetching available contract examples...");

        let repo_cache_path = self.ensure_cache_updated().await?;

        // Read examples from the cached repository
        let examples_path = repo_cache_path.join("examples");
        let mut examples = Vec::new();

        if examples_path.exists() {
            for entry in fs::read_dir(examples_path)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        examples.push(name.to_string());
                    }
                }
            }
            examples.sort();
        }

        eprintln!("\n📦 Available contract examples:");
        eprintln!("────────────────────────────────");

        for example in examples {
            eprintln!("  📁 {example}");
        }

        eprintln!("\n💡 Usage:");
        eprintln!("   stellar-registry contract generate --from <example-name>");
        eprintln!("   Example: stellar-registry contract generate --from nft-royalties");

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

    fn cache_repository(
        repo_cache_path: &Path,
        cache_ref_file: &Path,
        tag_name: &str,
    ) -> Result<(), Error> {
        fs::create_dir_all(repo_cache_path)?;

        // Use the specific tag instead of main branch
        let repo_ref = format!("OpenZeppelin/stellar-contracts#{tag_name}");
        degit::degit(&repo_ref, &repo_cache_path.to_string_lossy());

        if repo_cache_path.read_dir()?.next().is_none() {
            return Err(Error::GitCloneFailed(format!(
                "Failed to download repository release {tag_name} to cache"
            )));
        }

        fs::write(cache_ref_file, tag_name)?;
        Ok(())
    }

    fn copy_directory_contents(source: &Path, dest: &Path) -> Result<(), Error> {
        let copy_options = fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true);

        fs_extra::dir::copy(source, dest, &copy_options)
            .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(())
    }
}

fn open_wizard() -> Result<(), Error> {
    eprintln!("🧙 Opening OpenZeppelin Contract Wizard...");

    let url = "https://wizard.openzeppelin.com/stellar";

    webbrowser::open(url)
        .map_err(|e| Error::BrowserFailed(format!("Failed to open browser: {e}")))?;

    eprintln!("✅ Opened Contract Wizard in your default browser");
    eprintln!("\n📋 Instructions:");
    eprintln!("   1. Configure your contract in the wizard");
    eprintln!("   2. Click 'Download' to get your contract files");
    eprintln!("   3. Extract the downloaded ZIP file");
    eprintln!("   4. Move the contract folder to your contracts/ directory");
    eprintln!("   5. Add the contract to your workspace Cargo.toml if needed");
    eprintln!(
        "\n💡 The wizard will generate a complete Soroban contract with your selected features!"
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
    async fn test_ls_command() {
        let cmd = create_test_cmd(None, true, false);

        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/contents/examples",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"name": "example1", "type": "dir"}, {"name": "example2", "type": "dir"}]"#)
        .create();

        let result = cmd.run().await;
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
        let result = cmd.run().await;
        assert!(matches!(result, Err(Error::NoActionSpecified)));
    }
}
