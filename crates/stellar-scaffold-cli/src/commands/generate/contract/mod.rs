use clap::Parser;
use reqwest;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Deserialize)]
struct RefResponse {
    object: RefObject,
}

#[derive(Deserialize)]
struct RefObject {
    sha: String,
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

#[derive(Deserialize, Debug)]
struct GitHubContent {
    name: String,
    #[serde(rename = "type")]
    content_type: String,
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

    async fn clone_example(&self, example_name: &str) -> Result<(), Error> {
        eprintln!("🔍 Downloading example '{example_name}'...");

        let dest_path = self
            .output
            .clone()
            .unwrap_or_else(|| format!("contracts/{example_name}"));

        let cache_dir = dirs::cache_dir().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cache directory not found",
            ))
        })?;

        let repo_cache_path = cache_dir.join("stellar-scaffold-cli/openzeppelin-stellar-contracts");
        let cache_ref_file = repo_cache_path.join(".main_ref");

        // Get the current main branch ref (lightweight API call)
        let current_ref = Self::fetch_main_ref().await?;

        let should_update_cache = if repo_cache_path.exists() {
            if let Ok(cached_ref) = fs::read_to_string(&cache_ref_file) {
                if cached_ref.trim() == current_ref {
                    eprintln!("📂 Using cached repository (up to date)...");
                    false
                } else {
                    eprintln!("📂 Repository has updates. Refreshing cache...");
                    true
                }
            } else {
                eprintln!("📂 Cache metadata missing. Updating...");
                true
            }
        } else {
            eprintln!("📂 Cache not found. Downloading...");
            true
        };

        if should_update_cache {
            if repo_cache_path.exists() {
                fs::remove_dir_all(&repo_cache_path)?;
            }
            Self::cache_repository(&repo_cache_path, &cache_ref_file, &current_ref)?;
        }

        // Check if the example exists
        let example_source_path = repo_cache_path.join(format!("examples/{example_name}"));
        if !example_source_path.exists() {
            return Err(Error::ExampleNotFound(example_name.to_string()));
        }

        // Create destination and copy contents
        fs::create_dir_all(&dest_path)?;
        Self::copy_directory_contents(&example_source_path.to_string_lossy(), &dest_path)?;

        eprintln!("✅ Successfully downloaded example '{example_name}' to {dest_path}");
        Ok(())
    }

    async fn fetch_main_ref() -> Result<String, Error> {
        Self::fetch_main_ref_from_url(
            "https://api.github.com/repos/OpenZeppelin/stellar-contracts/git/refs/heads/main",
        )
        .await
    }

    async fn fetch_main_ref_from_url(url: &str) -> Result<String, Error> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "stellar-scaffold-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
        }

        let ref_data: RefResponse = response.json().await?;
        Ok(ref_data.object.sha)
    }

    fn cache_repository(
        repo_cache_path: &Path,
        cache_ref_file: &Path,
        current_ref: &str,
    ) -> Result<(), Error> {
        fs::create_dir_all(repo_cache_path)?;

        degit::degit(
            "OpenZeppelin/stellar-contracts",
            &repo_cache_path.to_string_lossy(),
        );

        if repo_cache_path.read_dir()?.next().is_none() {
            return Err(Error::GitCloneFailed(
                "Failed to download repository to cache".to_string(),
            ));
        }

        fs::write(cache_ref_file, current_ref)?;
        Ok(())
    }

    fn copy_directory_contents(source: &str, dest: &str) -> Result<(), Error> {
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let source_path = entry.path();
            let dest_path = Path::new(dest).join(entry.file_name());

            if source_path.is_dir() {
                std::fs::create_dir_all(&dest_path)?;
                Self::copy_directory_contents(
                    &source_path.to_string_lossy(),
                    &dest_path.to_string_lossy(),
                )?;
            } else {
                std::fs::copy(&source_path, &dest_path)?;
            }
        }
        Ok(())
    }

    async fn list_examples(&self) -> Result<(), Error> {
        eprintln!("📋 Fetching available contract examples...");

        let contents = self.fetch_example_names().await?;
        eprintln!("\n📦 Available contract examples:");
        eprintln!("────────────────────────────────");

        for item in contents {
            eprintln!("  📁 {item}");
        }

        eprintln!("\n💡 Usage:");
        eprintln!("   stellar-registry contract generate --from <example-name>");
        eprintln!("   Example: stellar-registry contract generate --from nft-royalties");

        Ok(())
    }

    async fn fetch_example_names(&self) -> Result<Vec<String>, Error> {
        self.fetch_example_names_from_url(
            "https://api.github.com/repos/OpenZeppelin/stellar-contracts/contents/examples",
        )
        .await
    }

    async fn fetch_example_names_from_url(&self, url: &str) -> Result<Vec<String>, Error> {
        let client = reqwest::Client::new();

        let response = client
            .get(url)
            .header("User-Agent", "stellar-registry-cli")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Reqwest(response.error_for_status().unwrap_err()));
        }

        let contents: Vec<GitHubContent> = response.json().await?;

        Ok(contents
            .into_iter()
            .filter(|item| item.content_type == "dir")
            .map(|item| item.name)
            .collect())
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
    use std::fs;
    use tempfile::tempdir;

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
    async fn test_fetch_main_ref() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/git/refs/heads/main",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "ref": "refs/heads/main",
                "object": {
                    "sha": "abc123def456789",
                    "type": "commit"
                }
            }"#,
        )
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/git/refs/heads/main",
            server_url()
        );
        let result = Cmd::fetch_main_ref_from_url(&mock_url).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123def456789");
    }

    #[tokio::test]
    async fn test_fetch_main_ref_error() {
        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/git/refs/heads/main",
        )
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message": "Not Found"}"#)
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/git/refs/heads/main",
            server_url()
        );
        let result = Cmd::fetch_main_ref_from_url(&mock_url).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_action_specified() {
        let cmd = create_test_cmd(None, false, false);
        let result = cmd.run().await;
        assert!(matches!(result, Err(Error::NoActionSpecified)));
    }

    #[tokio::test]
    async fn test_copy_directory_contents() {
        let temp = tempdir().unwrap();
        let source_dir = temp.path().join("source");
        let dest_dir = temp.path().join("dest");

        // Create source directory structure
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(dest_dir.clone()).unwrap();

        // Create test files
        fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        fs::write(source_dir.join("file2.txt"), "content2").unwrap();

        let subdir = source_dir.join("subdir");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("nested.txt"), "nested content").unwrap();

        // Test the copy operation
        let result = Cmd::copy_directory_contents(
            &source_dir.to_string_lossy(),
            &dest_dir.to_string_lossy(),
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("file1.txt").exists());
        assert!(dest_dir.join("file2.txt").exists());
        assert!(dest_dir.join("subdir/nested.txt").exists());

        // Verify content
        assert_eq!(
            fs::read_to_string(dest_dir.join("file1.txt")).unwrap(),
            "content1"
        );
        assert_eq!(
            fs::read_to_string(dest_dir.join("subdir/nested.txt")).unwrap(),
            "nested content"
        );
    }

    #[tokio::test]
    async fn test_fetch_example_names() {
        let cmd = create_test_cmd(None, false, false);

        let _m = mock(
            "GET",
            "/repos/OpenZeppelin/stellar-contracts/contents/examples",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[
                {"name": "nft-royalties", "type": "dir"}, 
                {"name": "ownable", "type": "dir"},
                {"name": "README.md", "type": "file"}
            ]"#,
        )
        .create();

        let mock_url = format!(
            "{}/repos/OpenZeppelin/stellar-contracts/contents/examples",
            server_url()
        );
        let result = cmd.fetch_example_names_from_url(&mock_url).await;

        assert!(result.is_ok());

        let examples = result.unwrap();
        assert_eq!(examples.len(), 2);
        assert!(examples.contains(&"nft-royalties".to_string()));
        assert!(examples.contains(&"ownable".to_string()));
        assert!(!examples.contains(&"README.md".to_string()));
    }
}
