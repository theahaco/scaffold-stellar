use std::{env, path::{Path, PathBuf}};

use assert_cmd::Command;
use stellar_cli::{
    CommandParser,
    commands::{self as cli, NetworkRunnable, contract::upload, global, network},
};

use crate::AssertExt;
use crate::common::{TestEnv, find_registry_wasm};

const RPC_URL: &str = "http://localhost:8000/soroban/rpc";

#[derive(Clone)]
pub struct RegistryTest {
    pub env: TestEnv,
    pub registry_address: String,
}

impl RegistryTest {
    pub async fn new() -> Self {
        let env = TestEnv::new_with_contracts("soroban-init-boilerplate", &["hello_world"]);
        Self::parse_cmd_internal::<network::add::Cmd>(
            &env,
            &[
                "localhost",
                "--rpc-url",
                RPC_URL,
                "--network-passphrase",
                "Standalone Network ; February 2017",
            ],
        )
        .unwrap()
        .run()
        .unwrap();
        Self::parse_cmd_internal::<network::default::Cmd>(&env, &["localhost"])
            .unwrap()
            .run(&global::Args::default())
            .unwrap();
        //let env = TestEnv::new("soroban-init-boilerplate");

        // Deploy registry contract
        let registry_address = Self::deploy_registry(&env).await;
        // Set environment variables for testnet configuration
        unsafe {
            env::set_var("STELLAR_RPC_URL", RPC_URL);
            env::set_var("STELLAR_ACCOUNT", "alice");
            env::set_var(
                "STELLAR_NETWORK_PASSPHRASE",
                "Standalone Network ; February 2017",
            );
            env::set_var("STELLAR_REGISTRY_CONTRACT_ID", &registry_address);
        };

        Self {
            env,
            registry_address,
        }
    }

    async fn deploy_registry(env: &TestEnv) -> String {
        // Set up environment with an account
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{RPC_URL}", network-passphrase = "Standalone Network ; February 2017"}}
accounts = ["alice"]
[development.contracts]
soroban_hello_world_contract.client = false
"#
        ));

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
        let hash = Self::parse_cmd_internal::<upload::Cmd>(
            env,
            &[
                "--wasm",
                wasm_path
                    .to_str()
                    .expect("we do not support non-utf8 paths"),
                "--source",
                "alice",
                "--rpc-url",
                RPC_URL,
                "--network-passphrase",
                "Standalone Network ; February 2017",
            ],
        )
        .expect("Failed to parse arguments for upload")
        .run_against_rpc_server(None, None)
        .await
        .expect("Failed to upload contract")
        .into_result()
        .expect("no hash returned by 'contract upload'")
        .to_string();

        eprintln!("🪞 Deploying registry contract...");

        // Deploy contract using the Stellar CLI library directly with alice account
        let deploy_args = [
            "--wasm-hash",
            &hash,
            "--source",
            "alice",
            "--rpc-url",
            RPC_URL,
            "--network-passphrase",
            "Standalone Network ; February 2017",
            "--",
            "--admin",
            "alice",
        ];
        let contract_id =
            Self::parse_cmd_internal::<cli::contract::deploy::wasm::Cmd>(env, &deploy_args)
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

    pub fn parse_cmd<T>(&self, s: &[&str]) -> Result<T, clap::Error>
    where
        T: CommandParser<T>,
    {
        Self::parse_cmd_internal(&self.env, s)
    }

    fn parse_cmd_internal<T>(env: &TestEnv, s: &[&str]) -> Result<T, clap::Error>
    where
        T: CommandParser<T>,
    {
        let mut cmd = s.to_vec();
        cmd.insert(0, &format!("--config-dir={}", config_dir(&env.cwd).to_str().unwrap()));
        T::parse_arg_vec(s)
    }

    pub fn registry_cli(&self, cmd: &str) -> Command {
        let mut registry = Command::cargo_bin("stellar-registry").unwrap();
        registry.current_dir(&self.env.cwd);
        registry.arg(cmd);
        registry.arg("--config-dir");
        registry.arg(config_dir(&self.env.cwd).to_str().unwrap());
        registry
    }
}

fn config_dir(p: &Path) -> PathBuf {
    p.join(".config").join("stellar")
}