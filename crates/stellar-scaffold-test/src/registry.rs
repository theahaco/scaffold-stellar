use crate::AssertExt;
use crate::common::{TestEnv, find_registry_wasm};
use assert_cmd::Command;
use std::path::PathBuf;
use stellar_cli::commands::NetworkRunnable;
use stellar_cli::{CommandParser, commands as cli};

#[derive(Clone)]
pub struct RegistryTest {
    pub env: TestEnv,
    pub registry_address: String,
}

impl RegistryTest {
    pub async fn new() -> Self {
        let env = TestEnv::new_with_contracts("soroban-init-boilerplate", &["hello_world"]);
        //let env = TestEnv::new("soroban-init-boilerplate");

        // Deploy registry contract
        let registry_address = Self::deploy_registry(&env).await;

        Self {
            env,
            registry_address,
        }
    }

    async fn deploy_registry(env: &TestEnv) -> String {
        // Set up environment with an account
        env.set_environments_toml(r#"
[development]
network = { rpc-url = "http://localhost:8000/rpc", network-passphrase = "Standalone Network ; February 2017"}
accounts = ["alice"]
[development.contracts]
soroban_hello_world_contract.client = false
"#);

        // Build contracts to generate wasm files
        let stderr = env
            .scaffold_build("development", true)
            .assert()
            .success()
            .stderr_as_str();
        eprintln!("{stderr}");

        eprintln!("📲 Installing registry contract wasm...");

        // Get wasm path
        let wasm_path = find_registry_wasm().unwrap();

        // Upload wasm using the Stellar CLI library directly with alice account
        let hash = cli::contract::upload::Cmd::parse_arg_vec(&[
            "--wasm",
            wasm_path
                .to_str()
                .expect("we do not support non-utf8 paths"),
            "--source",
            "alice",
            "--config-dir",
            env.cwd.to_str().unwrap(),
            "--rpc-url",
            "http://localhost:8000/soroban/rpc",
            "--network-passphrase",
            "Standalone Network ; February 2017",
        ])
        .expect("Failed to parse arguments for upload")
        .run_against_rpc_server(None, None)
        .await
        .expect("Failed to upload contract")
        .into_result()
        .expect("no hash returned by 'contract upload'")
        .to_string();

        eprintln!("🪞 Deploying registry contract...");

        // Deploy contract using the Stellar CLI library directly with alice account
        let deploy_args = vec![
            "--wasm-hash".to_string(),
            hash.clone(),
            "--source".to_string(),
            "alice".to_string(),
            "--config-dir".to_string(),
            env.cwd.to_str().unwrap().to_string(),
            "--rpc-url".to_string(),
            "http://localhost:8000/soroban/rpc".to_string(),
            "--network-passphrase".to_string(),
            "Standalone Network ; February 2017".to_string(),
            "--".to_string(),
            "--admin".to_string(),
            "alice".to_string(),
        ];
        let deploy_arg_refs: Vec<&str> = deploy_args.iter().map(String::as_str).collect();
        let contract_id = cli::contract::deploy::wasm::Cmd::parse_arg_vec(&deploy_arg_refs)
            .expect("Failed to parse arguments for deploy")
            .run_against_rpc_server(None, None)
            .await
            .expect("Failed to deploy contract")
            .into_result()
            .expect("no contract id returned by 'contract deploy'")
            .to_string()
            .trim()
            .to_string();

        eprintln!("✅ Registry deployed at: {contract_id}");

        contract_id
    }

    pub fn register_contract(&self, name: &str, wasm_path: &PathBuf) -> Command {
        // Add logic to register a contract
        let mut cmd = self.env.registry_cli("register");
        cmd.arg("--name").arg(name);
        cmd.arg("--wasm").arg(wasm_path);
        cmd
    }

    pub fn registry_cli(&self, cmd: &str) -> Command {
        let mut registry = Command::cargo_bin("stellar-registry").unwrap();
        registry.current_dir(&self.env.cwd);
        registry.arg(cmd);
        registry
    }
}
