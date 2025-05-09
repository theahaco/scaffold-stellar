#![allow(dead_code)]
use assert_cmd::{assert::Assert, Command};
use assert_fs::TempDir;
use fs_extra::dir::{copy, CopyOptions};
use std::env;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command as ProcessCommand;
use tokio::time::{sleep, timeout};
use tokio_stream::StreamExt;

pub struct TestEnv {
    pub temp_dir: TempDir,
    pub cwd: PathBuf,
}

pub trait AssertExt {
    #[allow(unused)]
    fn stdout_as_str(&self) -> String;
    fn stderr_as_str(&self) -> String;
}

impl AssertExt for Assert {
    fn stdout_as_str(&self) -> String {
        String::from_utf8(self.get_output().stdout.clone())
            .expect("failed to make str")
            .trim()
            .to_owned()
    }
    fn stderr_as_str(&self) -> String {
        String::from_utf8(self.get_output().stderr.clone())
            .expect("failed to make str")
            .trim()
            .to_owned()
    }
}

impl TestEnv {
    pub fn new(template: &str) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

        copy(template_dir.join(template), &temp_dir, &CopyOptions::new()).unwrap();

        Self {
            cwd: temp_dir.path().join(template),
            temp_dir,
        }
    }

    pub fn new_empty() -> Self {
        let temp_dir = TempDir::new().unwrap();
        Self {
            cwd: temp_dir.path().to_path_buf(),
            temp_dir,
        }
    }

    pub fn from<F: FnOnce(&TestEnv)>(template: &str, f: F) {
        let test_env = TestEnv::new(template);
        f(&test_env);
    }

    pub async fn from_async<F, Fut>(template: &str, f: F)
    where
        F: FnOnce(TestEnv) -> Fut,
        Fut: Future<Output = ()>,
    {
        let test_env = TestEnv::new(template);
        f(test_env).await;
    }

    pub async fn wait_for_output<
        T: tokio_stream::Stream<Item = Result<String, tokio::io::Error>> + Unpin,
    >(
        lines: &mut T,
        expected: &str,
    ) {
        let timeout_duration = Duration::from_secs(240); // 4 minutes
        let result = timeout(timeout_duration, async {
            loop {
                match lines.next().await {
                    Some(Ok(line)) => {
                        println!("{line}");
                        if line.contains(expected) {
                            return;
                        }
                    }
                    Some(Err(e)) => println!("Error reading line: {e:?}"),
                    None => {
                        sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        })
        .await;
        match result {
            Ok(()) => {
                println!("Found string {expected}");
            }
            _ => panic!("Timed out waiting for output: {expected}"),
        }
    }

    pub fn modify_file(&self, path: &str, content: &str) {
        let file_path = self.cwd.join(path);
        std::fs::write(file_path, content).expect("Failed to modify file");
    }

    pub fn delete_file(&self, path: &str) {
        let file_path = self.cwd.join(path);
        std::fs::remove_file(file_path).expect("Failed to delete file");
    }

    pub fn scaffold_build(&self, env: &str, randomize_wasm: bool) -> Command {
        let mut stellar_scaffold = Command::cargo_bin("stellar-scaffold").unwrap();
        stellar_scaffold.current_dir(&self.cwd);
        stellar_scaffold.arg("build");
        stellar_scaffold.arg(env);
        stellar_scaffold.arg("--build-clients");
        
        if randomize_wasm {
            // Add a random meta key-value pair to make the WASM unique
            let random_value = uuid::Uuid::new_v4().to_string();
            stellar_scaffold.arg("--meta");
            stellar_scaffold.arg(format!("random_test={}", random_value));
        }
        
        stellar_scaffold
    }

    fn cargo_bin_stellar_scaffold() -> PathBuf {
        assert_cmd::cargo::cargo_bin("stellar-scaffold")
    }

    pub fn stellar_scaffold_process(&self, cmd: &str, additional_args: &[&str]) -> ProcessCommand {
        let bin = Self::cargo_bin_stellar_scaffold();
        println!("{}", bin.display());
        let mut stellar_scaffold = ProcessCommand::new(bin);
        stellar_scaffold.current_dir(&self.cwd);
        stellar_scaffold.arg(cmd);
        for arg in additional_args {
            stellar_scaffold.arg(arg);
        }
        stellar_scaffold
    }

    pub fn scaffold(&self, cmd: &str) -> Command {
        if cmd == "build" {
            self.scaffold_build("development", true)
        } else {
            let mut stellar_scaffold = Command::cargo_bin("stellar-scaffold").unwrap();
            stellar_scaffold.current_dir(&self.cwd);
            stellar_scaffold.arg(cmd);
            stellar_scaffold
        }
    }

    pub fn stellar_scaffold_env(&self, env: &str, randomize_wasm: bool) -> Command {
        self.scaffold_build(env, randomize_wasm)
    }

    pub fn stellar(&self, cmd: &str) -> Command {
        let mut stellar = Command::new("stellar");
        stellar.env(
            "PATH",
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/bin"),
        );
        stellar.current_dir(&self.cwd);
        stellar.arg(cmd);
        stellar
    }

    pub fn set_environments_toml(&self, contents: impl AsRef<[u8]>) {
        std::fs::write(self.cwd.join("environments.toml"), contents).unwrap();
    }

    pub fn switch_to_new_directory(
        &mut self,
        template: &str,
        new_dir_name: &str,
    ) -> std::io::Result<()> {
        let new_dir = self.temp_dir.path().join(new_dir_name);
        fs::create_dir_all(&new_dir)?;
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        copy(template_dir.join(template), &new_dir, &CopyOptions::new()).unwrap();
        self.cwd = new_dir.join(template);
        Ok(())
    }
}

pub fn find_binary(name: &str) -> Option<PathBuf> {
    let exe_path = env::current_exe().ok()?;
    let project_root = find_project_root(&exe_path)?;
    Some(project_root.join("target").join("bin").join(name))
}

fn find_project_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = start_path;
    while let Some(parent) = current.parent() {
        if parent.join("Cargo.toml").exists() {
            return Some(parent.to_path_buf());
        }
        current = parent;
    }
    None
}
