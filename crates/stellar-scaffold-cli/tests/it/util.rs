#![allow(dead_code)]
use assert_cmd::{assert::Assert, Command};
use assert_fs::TempDir;
use fs_extra::dir::{copy, CopyOptions};
use rand::{thread_rng, Rng};
use std::env;
use std::error::Error;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command as ProcessCommand;
use tokio::time::{sleep, timeout};
use tokio_stream::StreamExt;
use toml::Value;

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

    pub fn modify_wasm(&self, contract_name: &str) -> Result<(), Box<dyn Error>> {
        // Read Cargo.toml to get the actual name
        let cargo_toml_path = self
            .cwd
            .join("contracts")
            .join(contract_name)
            .join("Cargo.toml");
        let cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
        let cargo_toml: Value = toml::from_str(&cargo_toml_content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let package_name = cargo_toml["package"]["name"].as_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid Cargo.toml")
        })?;

        // Convert package name to proper filename format
        let filename = package_name.replace('-', "_");

        let wasm_path = self.cwd.join(format!("target/stellar/{filename}.wasm"));
        let mut wasm_bytes = fs::read(&wasm_path)?;
        let mut rng = thread_rng();
        let random_bytes: Vec<u8> = (0..10).map(|_| rng.gen()).collect();
        wasm_gen::write_custom_section(&mut wasm_bytes, "random_data", &random_bytes);
        fs::write(&wasm_path, wasm_bytes)?;
        Ok(())
    }

    pub fn scaffold_build(&self, env: &str, randomize_wasm: bool) -> Command {
        if randomize_wasm {
            // Run initial build
            let mut initial_build = Command::cargo_bin("stellar-scaffold").unwrap();
            initial_build.current_dir(&self.cwd);
            initial_build.arg("build");
            initial_build.arg(env);
            initial_build
                .output()
                .expect("Failed to execute initial build");

            // Modify WASM files
            let contracts_dir = self.cwd.join("contracts");
            if let Ok(entries) = fs::read_dir(contracts_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            if let Some(contract_name) = entry.file_name().to_str() {
                                self.modify_wasm(contract_name)
                                    .expect("Failed to modify WASM");
                            }
                        }
                    }
                }
            }
        }
        // Run final build with --build-clients
        let mut stellar_scaffold = Command::cargo_bin("stellar-scaffold").unwrap();
        stellar_scaffold.current_dir(&self.cwd);
        stellar_scaffold.arg("build");
        stellar_scaffold.arg(env);
        stellar_scaffold.arg("--build-clients");
        stellar_scaffold
    }

    fn cargo_bin_stellar_scaffold() -> PathBuf {
        PathBuf::from(std::env::var("CARGO_BIN_EXE_stellar_scaffold").unwrap())
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

    pub fn scaffold_env(&self, env: &str, randomize_wasm: bool) -> Command {
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
