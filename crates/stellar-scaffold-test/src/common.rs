#![allow(dead_code)]
#![allow(deprecated)]
use assert_cmd::{Command, assert::Assert};
use assert_fs::TempDir;
use fs_extra::dir::{CopyOptions, copy};
use std::env;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command as ProcessCommand;
use tokio::time::{sleep, timeout};
use tokio_stream::StreamExt;

#[derive(Clone)]
pub struct TestEnv {
    pub temp_dir: Arc<TempDir>,
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
        let temp_dir = Arc::new(TempDir::new().unwrap());
        let cwd = temp_dir.path().join(template);
        Self::set_options(&temp_dir);
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");

        copy(template_dir.join(template), &*temp_dir, &CopyOptions::new()).unwrap();

        Self { temp_dir, cwd }
    }

    pub fn new_with_contracts(template: &str, contract_names: &[&str]) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let source_path = template_dir.join(template);
        let dest_path = temp_dir.path().join(template);

        // First, copy everything
        let mut copy_options = CopyOptions::new();
        copy_options.skip_exist = true;
        copy_options.content_only = true;

        copy(&source_path, &dest_path, &copy_options).unwrap();

        // Remove the contracts directory entirely
        let contracts_dir = dest_path.join("contracts");
        if contracts_dir.exists() {
            std::fs::remove_dir_all(&contracts_dir).unwrap();
            std::fs::create_dir(&contracts_dir).unwrap();
        }

        for contract_name in contract_names {
            let source_contract = source_path.join("contracts").join(contract_name);
            copy(&source_contract, &contracts_dir, &CopyOptions::new()).unwrap();
        }

        Self {
            cwd: dest_path,
            temp_dir: temp_dir.into(),
        }
    }

    pub fn set_options(temp_dir: &TempDir) {
        unsafe {
            std::env::set_var(
                "XDG_CACHE_DIR",
                temp_dir.path().join(".cache").to_str().unwrap(),
            );
            std::env::set_var(
                "XDG_CONFIG_HOME",
                temp_dir.path().join(".config").to_str().unwrap(),
            );
        }
    }

    pub fn new_empty() -> Self {
        let temp_dir = Arc::new(TempDir::new().unwrap());
        let cwd = temp_dir.path().to_path_buf();
        eprintln!("new test dir created at {}", temp_dir.to_str().unwrap());
        Self::set_options(&temp_dir);
        Self { temp_dir, cwd }
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

    pub async fn init_project(&mut self, project_name: &str) -> std::io::Result<()> {
        let init_output = ProcessCommand::new(Self::cargo_bin_stellar_scaffold())
            .arg("init")
            .arg(project_name)
            .current_dir(self.temp_dir.path())
            .output()
            .await
            .expect("Failed to run scaffold init");

        assert!(
            init_output.status.success(),
            "scaffold init failed: {}",
            String::from_utf8_lossy(&init_output.stderr)
        );

        // Update the cwd to point to the newly created project
        self.cwd = self.temp_dir.path().join(project_name);
        Ok(())
    }

    pub async fn from_init<F, Fut>(project_name: &str, f: F)
    where
        F: FnOnce(TestEnv) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut test_env = TestEnv::new_empty();
        test_env
            .init_project(project_name)
            .await
            .expect("Failed to init project");
        test_env.cwd = test_env.temp_dir.path().join(project_name);
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
        stellar_scaffold.env(
            "XDG_CONFIG_HOME",
            self.cwd.join(".config").to_str().unwrap(),
        );
        stellar_scaffold.env("RUST_LOG", "trace");

        if randomize_wasm {
            // Add a random meta key-value pair to make the WASM unique
            let random_value = uuid::Uuid::new_v4().to_string();
            stellar_scaffold.arg("--meta");
            stellar_scaffold.arg(format!("random_test={random_value}"));
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

    pub fn config_dir(&self) -> PathBuf {
        self.cwd.join(".config").join("stellar")
    }

    pub fn stellar_scaffold_custom_dir(
        &self,
        cmd: &str,
        additional_args: &[&str],
        dir: &PathBuf,
    ) -> Command {
        let bin = Self::cargo_bin_stellar_scaffold();
        let mut stellar_scaffold = Command::new(bin);
        stellar_scaffold.current_dir(dir);
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
            stellar_scaffold.env("XDG_CACHE_DIR", self.cwd.join(".cache").to_str().unwrap());
            stellar_scaffold.env(
                "XDG_CONFIG_HOME",
                self.cwd.join(".config").to_str().unwrap(),
            );
            stellar_scaffold.arg(cmd);
            stellar_scaffold
        }
    }

    pub fn stellar(&self, cmd: &str) -> Command {
        let mut stellar = Command::new("stellar");
        stellar.env(
            "PATH",
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/bin"),
        );
        stellar.env(
            "XDG_CONFIG_HOME",
            self.cwd.join(".config").to_str().unwrap(),
        );
        stellar.current_dir(&self.cwd);
        stellar.arg(cmd);
        stellar
    }

    pub fn set_environments_toml(&self, contents: impl AsRef<[u8]>) {
        std::fs::write(self.cwd.join("environments.toml"), contents).unwrap();
    }

    pub fn copy_env(&self) {
        std::fs::copy(self.cwd.join(".env.example"), self.cwd.join(".env")).unwrap();
    }

    pub fn switch_to_new_directory(
        &mut self,
        template: &str,
        new_dir_name: &str,
    ) -> std::io::Result<()> {
        let new_dir = self.temp_dir.path().join(new_dir_name);
        fs::create_dir_all(&new_dir)?;
        let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        copy(template_dir.join(template), &new_dir, &CopyOptions::new()).unwrap();
        self.cwd = new_dir.join(template);
        Ok(())
    }

    pub fn registry_cli(&self, cmd: &str) -> Command {
        let mut registry = Command::cargo_bin("stellar-registry").unwrap();
        registry.current_dir(&self.cwd);
        registry.arg(cmd);
        registry
    }

    pub fn update_package_json_to_use_built_binary(
        &self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let package_json_path = self.cwd.join("package.json");
        let stellar_bin = Self::cargo_bin_stellar_scaffold();
        eprintln!("using stellar scaffold binary {}", stellar_bin.display());

        let package_json_content = std::fs::read_to_string(&package_json_path)?;

        let updated_content = package_json_content.replace(
            "stellar scaffold watch --build-clients",
            &format!("{} watch --build-clients", stellar_bin.display()),
        );

        std::fs::write(&package_json_path, updated_content)?;

        Ok(())
    }
}

pub fn find_binary(name: &str) -> Option<PathBuf> {
    let exe_path = env::current_exe().ok()?;
    let project_root = find_project_root(&exe_path)?;
    Some(project_root.join("target").join("bin").join(name))
}

pub fn find_stellar_wasm_dir() -> Option<PathBuf> {
    let exe_path = env::current_exe().ok()?;
    let project_root = find_project_root(&exe_path)?;
    Some(project_root.join("target").join("stellar").join("local"))
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
