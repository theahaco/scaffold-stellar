#![allow(deprecated)]
use assert_cmd::Command;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use stellar_cli::{
    CommandParser,
    commands::{self as cli, NetworkRunnable, contract::upload, global, keys, network},
};

use crate::common::{TestEnv, find_stellar_wasm_dir};

#[derive(Clone)]
pub struct RegistryTest {
    pub env: TestEnv,
    pub registry_address: String,
}

impl RegistryTest {
    pub async fn new() -> Self {
        let env = TestEnv::new_empty();
        let rpc_url = &crate::rpc_url();
        Self::parse_cmd_internal::<network::add::Cmd>(
            &env,
            &[
                "localhost",
                "--rpc-url",
                rpc_url,
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
        // Set environment variables for testnet configuration
        unsafe {
            env::set_var("STELLAR_RPC_URL", rpc_url);
            env::set_var("STELLAR_ACCOUNT", "alice");
            env::set_var(
                "STELLAR_NETWORK_PASSPHRASE",
                "Standalone Network ; February 2017",
            );
        };
        let registry_address = Self::deploy_registry(&env).await;
        unsafe {
            env::set_var("STELLAR_REGISTRY_CONTRACT_ID", &registry_address);
        }

        Self {
            env,
            registry_address,
        }
    }

    async fn deploy_registry(env: &TestEnv) -> String {
        let rpc_url = &crate::rpc_url();
        Self::parse_cmd_internal::<keys::generate::Cmd>(env, &["alice", "--fund"])
            .unwrap()
            .run(&global::Args::default())
            .await
            .unwrap();

        eprintln!("📲 Installing registry contract wasm...");

        // Get wasm path
        let wasm_path = RandomizedWasm::new("registry.wasm").randomize(&env.cwd);
        println!("Wasm path: {:?}", wasm_path.display());
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
                rpc_url,
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
            rpc_url,
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
        let config_dir = format!("--config-dir={}", config_dir(&env.cwd).to_str().unwrap());
        cmd.insert(0, &config_dir);
        T::parse_arg_vec(&cmd)
    }

    pub fn registry_cli(&self, cmd: &str) -> Command {
        let mut registry = Command::cargo_bin("stellar-registry").unwrap();
        registry.current_dir(&self.env.cwd);
        registry.arg(cmd);
        registry.arg("--config-dir");
        registry.arg(config_dir(&self.env.cwd).to_str().unwrap());
        registry
    }
    pub fn hello_wasm_v1(&self) -> PathBuf {
        RandomizedWasm::new("hello_v1.wasm").randomize(&self.env.cwd)
    }

    pub fn hello_wasm_v2(&self) -> PathBuf {
        RandomizedWasm::new("hello_v2.wasm").randomize(&self.env.cwd)
    }
}

fn config_dir(p: &Path) -> PathBuf {
    p.join(".config").join("stellar")
}

struct RandomizedWasm(PathBuf);

impl RandomizedWasm {
    pub fn new(name: &str) -> Self {
        Self(PathBuf::from(name))
    }
    pub fn randomize(&self, temp_dir: &Path) -> PathBuf {
        let mut wasm_bytes = fs::read(find_stellar_wasm_dir().unwrap().join(&self.0))
            .expect("Failed to read wasm file");
        wasm_gen::write_custom_section(
            &mut wasm_bytes,
            "test_section",
            uuid::Uuid::new_v4().as_bytes(),
        );
        let out_file = temp_dir.join(&self.0);
        fs::write(&out_file, wasm_bytes).expect("Failed to write wasm file with custom section");
        out_file
    }
}
