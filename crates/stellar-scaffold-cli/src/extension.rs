use std::path::PathBuf;

use stellar_cli::print::Print;
use stellar_scaffold_ext_types::ExtensionManifest;

use crate::commands::build::env_toml::ExtensionEntry;

/// A fully validated, ready-to-invoke extension.
#[derive(Debug, Clone)]
pub struct ResolvedExtension {
    /// Name as declared in `environments.toml` (e.g. `"reporter"`).
    pub name: String,
    /// Absolute path to the `stellar-scaffold-<name>` binary.
    pub binary: PathBuf,
    /// Parsed manifest returned by `stellar-scaffold-<name> manifest`.
    pub manifest: ExtensionManifest,
    /// Per-extension config from `[env.ext.<name>]`, if provided.
    pub config: Option<serde_json::Value>,
}

/// Resolves each entry in `entries` to a [`ResolvedExtension`] by finding the
/// binary on `PATH`, invoking `<binary> manifest`, and parsing the output.
///
/// Missing binaries, failed invocations, and malformed manifests are each
/// warned and skipped — this never fails the overall build. The returned list
/// preserves the input order, minus any entries that could not be resolved.
pub fn discover(entries: &[ExtensionEntry], printer: &Print) -> Vec<ResolvedExtension> {
    let search_dirs = path_dirs();
    discover_in(entries, printer, &search_dirs)
}

fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default()
}

fn find_binary(name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let binary_name = binary_name(name);
    search_dirs
        .iter()
        .map(|dir| dir.join(&binary_name))
        .find(|p| p.is_file())
}

#[cfg(windows)]
fn binary_name(name: &str) -> String {
    format!("stellar-scaffold-{name}.exe")
}

#[cfg(not(windows))]
fn binary_name(name: &str) -> String {
    format!("stellar-scaffold-{name}")
}

fn discover_in(
    entries: &[ExtensionEntry],
    printer: &Print,
    search_dirs: &[PathBuf],
) -> Vec<ResolvedExtension> {
    let mut resolved = Vec::new();

    for entry in entries {
        let name = &entry.name;
        let binary_name = binary_name(name);

        let Some(binary) = find_binary(name, search_dirs) else {
            printer.warnln(format!(
                "Extension {name:?}: binary {binary_name:?} not found on PATH, skipping"
            ));
            continue;
        };

        let output = match std::process::Command::new(&binary).arg("manifest").output() {
            Ok(output) => output,
            Err(e) => {
                printer.warnln(format!(
                    "Extension {name:?}: failed to run `{binary_name} manifest`: {e}, skipping"
                ));
                continue;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            printer.warnln(format!(
                "Extension {name:?}: `{binary_name} manifest` exited with {}: {stderr}skipping",
                output.status
            ));
            continue;
        }

        let manifest: ExtensionManifest = match serde_json::from_slice(&output.stdout) {
            Ok(m) => m,
            Err(e) => {
                printer.warnln(format!(
                    "Extension {name:?}: malformed manifest from `{binary_name} manifest`: \
                     {e}, skipping"
                ));
                continue;
            }
        };

        resolved.push(ResolvedExtension {
            name: name.clone(),
            binary,
            manifest,
            config: entry.config.clone(),
        });
    }

    if !resolved.is_empty() {
        let names: Vec<&str> = resolved.iter().map(|e| e.name.as_str()).collect();
        printer.infoln(format!("Registered extensions: {}", names.join(", ")));
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printer() -> Print {
        Print::new(true) // quiet — we assert on return values, not output
    }

    fn entry(name: &str) -> ExtensionEntry {
        ExtensionEntry {
            name: name.to_owned(),
            config: None,
        }
    }

    fn entry_with_config(name: &str, config: serde_json::Value) -> ExtensionEntry {
        ExtensionEntry {
            name: name.to_owned(),
            config: Some(config),
        }
    }

    /// Write a shell script to `dir/<binary_name>` and make it executable.
    #[cfg(unix)]
    fn make_script(dir: &tempfile::TempDir, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.path().join(binary_name(name));
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Script that echoes a valid manifest JSON and exits 0.
    #[cfg(unix)]
    fn valid_manifest_script(dir: &tempfile::TempDir, name: &str, hooks: &[&str]) {
        let hooks_json = hooks
            .iter()
            .map(|h| format!("\"{h}\""))
            .collect::<Vec<_>>()
            .join(",");
        make_script(
            dir,
            name,
            &format!(r#"echo '{{"name":"{name}","version":"1.0.0","hooks":[{hooks_json}]}}'"#),
        );
    }

    #[test]
    #[cfg(unix)]
    fn discovers_valid_extension() {
        let dir = tempfile::TempDir::new().unwrap();
        valid_manifest_script(&dir, "reporter", &["post-compile", "post-deploy"]);

        let entries = vec![entry("reporter")];
        let result = discover_in(&entries, &printer(), &[dir.path().to_path_buf()]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "reporter");
        assert_eq!(result[0].manifest.name, "reporter");
        assert_eq!(
            result[0].manifest.hooks,
            vec!["post-compile", "post-deploy"]
        );
        assert!(result[0].config.is_none());
    }

    #[test]
    #[cfg(unix)]
    fn passes_config_through_to_resolved() {
        let dir = tempfile::TempDir::new().unwrap();
        valid_manifest_script(&dir, "reporter", &["post-compile"]);

        let config = serde_json::json!({ "warn_size_kb": 128 });
        let entries = vec![entry_with_config("reporter", config.clone())];
        let result = discover_in(&entries, &printer(), &[dir.path().to_path_buf()]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].config, Some(config));
    }

    #[test]
    fn skips_missing_binary() {
        let dir = tempfile::TempDir::new().unwrap();
        // No binary written to dir.

        let entries = vec![entry("missing")];
        let result = discover_in(&entries, &printer(), &[dir.path().to_path_buf()]);

        assert!(result.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn skips_failing_manifest_subcommand() {
        let dir = tempfile::TempDir::new().unwrap();
        make_script(&dir, "bad-exit", "exit 1");

        let entries = vec![entry("bad-exit")];
        let result = discover_in(&entries, &printer(), &[dir.path().to_path_buf()]);

        assert!(result.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn skips_malformed_manifest_json() {
        let dir = tempfile::TempDir::new().unwrap();
        make_script(&dir, "bad-json", "echo 'not valid json'");

        let entries = vec![entry("bad-json")];
        let result = discover_in(&entries, &printer(), &[dir.path().to_path_buf()]);

        assert!(result.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn preserves_order_and_skips_bad_entries() {
        let dir = tempfile::TempDir::new().unwrap();
        valid_manifest_script(&dir, "first", &["pre-compile"]);
        // "missing" has no binary.
        valid_manifest_script(&dir, "third", &["post-compile"]);

        let entries = vec![entry("first"), entry("missing"), entry("third")];
        let result = discover_in(&entries, &printer(), &[dir.path().to_path_buf()]);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "first");
        assert_eq!(result[1].name, "third");
    }
}
