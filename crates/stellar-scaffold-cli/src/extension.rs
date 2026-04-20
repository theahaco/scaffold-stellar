use std::io::Write as _;
use std::path::PathBuf;

use stellar_cli::print::Print;
use stellar_scaffold_ext_types::{ExtensionManifest, HookName};
use tokio::io::AsyncWriteExt as _;

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

/// Runs a single lifecycle hook across all registered extensions.
///
/// For each extension whose manifest lists `hook`, spawns
/// `stellar-scaffold-<name> <hook>` as a subprocess, serializes `context`
/// as JSON with `config` injected from the extension's entry in
/// `environments.toml`, writes the result to stdin, waits for it to exit,
/// then forwards its stdout to Scaffold's own stdout.
///
/// Non-zero exits are logged as errors but do not abort the loop — all
/// extensions are given a chance to run regardless of whether an earlier one
/// failed. The function itself is infallible from the caller's perspective.
pub async fn run_hook<C: serde::Serialize>(
    extensions: &[ResolvedExtension],
    hook: HookName,
    context: &C,
    printer: &Print,
) {
    let hook_str = hook.as_str();

    // Serialize context to a Value once; per-extension config is injected
    // into the object before writing to each extension's stdin.
    let ctx_value = match serde_json::to_value(context) {
        Ok(v) => v,
        Err(e) => {
            printer.errorln(format!(
                "Extension hook {hook_str:?}: failed to serialize context: {e}"
            ));
            return;
        }
    };

    for ext in extensions {
        if !ext.manifest.hooks.iter().any(|h| h == hook_str) {
            continue;
        }

        let input_json = match inject_config(&ctx_value, ext.config.as_ref()) {
            Ok(bytes) => bytes,
            Err(e) => {
                printer.errorln(format!(
                    "Extension {:?} hook {hook_str:?}: failed to serialize input: {e}",
                    ext.name
                ));
                continue;
            }
        };

        let binary_name = binary_name(&ext.name);

        let mut child = match tokio::process::Command::new(&ext.binary)
            .arg(hook_str)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                printer.errorln(format!(
                    "Extension {:?} hook {hook_str:?}: failed to spawn \
                     `{binary_name}`: {e}",
                    ext.name
                ));
                continue;
            }
        };

        // Write input JSON then shut down stdin so the child sees EOF.
        // Dropping without shutdown() could leave the pipe open on some
        // platforms, causing the child to block waiting for more input.
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(&input_json).await {
                printer.errorln(format!(
                    "Extension {:?} hook {hook_str:?}: failed to write context \
                     to stdin: {e}",
                    ext.name
                ));
                let _ = child.kill().await;
                continue;
            }
            let _ = stdin.shutdown().await;
        }

        let output = match child.wait_with_output().await {
            Ok(output) => output,
            Err(e) => {
                printer.errorln(format!(
                    "Extension {:?} hook {hook_str:?}: failed to wait for \
                     `{binary_name}`: {e}",
                    ext.name
                ));
                continue;
            }
        };

        // Forward the extension's stdout verbatim to Scaffold's stdout so
        // extensions can emit progress, JSON payloads, or human-readable
        // output without any added formatting.
        //
        // Flush immediately: the process may exit via std::process::exit()
        // which skips Drop and leaves BufWriter contents unwritten.
        if !output.stdout.is_empty() {
            let _ = std::io::stdout().write_all(&output.stdout);
            let _ = std::io::stdout().flush();
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            printer.errorln(format!(
                "Extension {:?} hook {hook_str:?}: `{binary_name}` exited \
                 with {}: {stderr}",
                ext.name, output.status
            ));
            // Continue — give remaining extensions a chance to run.
        }
    }
}

/// Injects `config` into a serialized context value and returns the combined
/// JSON bytes written to an extension's stdin.
///
/// `ctx` must be a JSON object (the output of serializing any context type).
/// The `config` key is set to the extension's config value, or `null` if the
/// extension has no config entry in `environments.toml`.
fn inject_config(
    ctx: &serde_json::Value,
    config: Option<&serde_json::Value>,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut map = match ctx {
        serde_json::Value::Object(m) => m.clone(),
        _ => serde_json::Map::new(),
    };
    map.insert(
        "config".to_string(),
        config.cloned().unwrap_or(serde_json::Value::Null),
    );
    serde_json::to_vec(&serde_json::Value::Object(map))
}

/// The resolved status of a single extension entry, used by `ext ls`.
#[derive(Debug)]
pub enum ExtensionListStatus {
    /// Binary found and manifest parsed successfully.
    Found { version: String, hooks: Vec<String> },
    /// Binary `stellar-scaffold-<name>` not found on PATH.
    MissingBinary,
    /// Binary found but `manifest` subcommand failed or returned malformed JSON.
    ManifestError(String),
}

/// Per-entry result returned by [`list`].
#[derive(Debug)]
pub struct ExtensionListEntry {
    pub name: String,
    pub status: ExtensionListStatus,
}

/// Returns one [`ExtensionListEntry`] per entry in `entries`, including entries
/// whose binary is missing or whose manifest is broken. Unlike [`discover`],
/// this never skips entries — it is intended for display, not for hook dispatch.
pub fn list(entries: &[ExtensionEntry]) -> Vec<ExtensionListEntry> {
    list_in(entries, &path_dirs())
}

fn list_in(entries: &[ExtensionEntry], search_dirs: &[PathBuf]) -> Vec<ExtensionListEntry> {
    entries
        .iter()
        .map(|entry| {
            let name = &entry.name;
            let Some(binary) = find_binary(name, search_dirs) else {
                return ExtensionListEntry {
                    name: name.clone(),
                    status: ExtensionListStatus::MissingBinary,
                };
            };

            let output = match std::process::Command::new(&binary).arg("manifest").output() {
                Err(e) => {
                    return ExtensionListEntry {
                        name: name.clone(),
                        status: ExtensionListStatus::ManifestError(e.to_string()),
                    };
                }
                Ok(o) => o,
            };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return ExtensionListEntry {
                    name: name.clone(),
                    status: ExtensionListStatus::ManifestError(stderr),
                };
            }

            match serde_json::from_slice::<ExtensionManifest>(&output.stdout) {
                Err(e) => ExtensionListEntry {
                    name: name.clone(),
                    status: ExtensionListStatus::ManifestError(e.to_string()),
                },
                Ok(manifest) => ExtensionListEntry {
                    name: name.clone(),
                    status: ExtensionListStatus::Found {
                        version: manifest.version,
                        hooks: manifest.hooks,
                    },
                },
            }
        })
        .collect()
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
    use stellar_scaffold_ext_types::HookName;

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

    // -----------------------------------------------------------------------
    // run_hook tests
    // -----------------------------------------------------------------------

    /// Build a `ResolvedExtension` directly, bypassing discovery.
    #[cfg(unix)]
    fn make_resolved(name: &str, binary: PathBuf, hooks: &[&str]) -> ResolvedExtension {
        ResolvedExtension {
            name: name.to_owned(),
            binary,
            manifest: ExtensionManifest {
                name: name.to_owned(),
                version: "1.0.0".to_owned(),
                hooks: hooks.iter().map(|h| (*h).to_string()).collect(),
            },
            config: None,
        }
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn run_hook_sends_context_to_stdin() {
        let dir = tempfile::TempDir::new().unwrap();
        // Script writes whatever it receives on stdin into received.json
        // next to the script itself.
        make_script(&dir, "reporter", r#"cat > "$(dirname "$0")/received.json""#);

        #[derive(serde::Serialize)]
        #[allow(clippy::items_after_statements)]
        struct Ctx {
            env: String,
        }
        let exts = vec![make_resolved(
            "reporter",
            dir.path().join(binary_name("reporter")),
            &["post-compile"],
        )];
        run_hook(
            &exts,
            HookName::PostCompile,
            &Ctx {
                env: "development".to_owned(),
            },
            &printer(),
        )
        .await;

        let received = std::fs::read_to_string(dir.path().join("received.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&received).unwrap();
        assert_eq!(parsed["env"], "development");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn run_hook_skips_extension_not_registered_for_hook() {
        let dir = tempfile::TempDir::new().unwrap();
        // Script creates a sentinel file when invoked.
        make_script(&dir, "reporter", r#"touch "$(dirname "$0")/was_invoked""#);
        let exts = vec![make_resolved(
            "reporter",
            dir.path().join(binary_name("reporter")),
            &["post-compile"], // registered for post-compile, not post-deploy
        )];
        run_hook(
            &exts,
            HookName::PostDeploy,
            &serde_json::json!({}),
            &printer(),
        )
        .await;

        assert!(!dir.path().join("was_invoked").exists());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn run_hook_continues_after_non_zero_exit() {
        let dir = tempfile::TempDir::new().unwrap();
        // First extension: exits 1, writes nothing.
        make_script(&dir, "failing", "exit 1");
        // Second extension: writes received context to a file.
        make_script(
            &dir,
            "succeeding",
            r#"cat > "$(dirname "$0")/received.json""#,
        );

        let exts = vec![
            make_resolved(
                "failing",
                dir.path().join(binary_name("failing")),
                &["post-compile"],
            ),
            make_resolved(
                "succeeding",
                dir.path().join(binary_name("succeeding")),
                &["post-compile"],
            ),
        ];

        run_hook(
            &exts,
            HookName::PostCompile,
            &serde_json::json!({ "env": "test" }),
            &printer(),
        )
        .await;

        // The second extension ran despite the first one failing.
        assert!(dir.path().join("received.json").exists());
    }
}
