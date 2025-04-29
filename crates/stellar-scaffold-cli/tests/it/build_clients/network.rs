use crate::util::{AssertExt, TestEnv};

#[test]
fn run_network_from_rpc_and_passphrase() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(
            r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
hello_world.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
"#,
        );

        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        assert!(stderr.contains("🌐 using network at http://localhost:8000/rpc\n"));
    });
}

#[test]
fn run_named_network() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        // create a network named "lol"
        env.stellar("network")
            .args([
                "add",
                "lol",
                "--rpc-url",
                "http://localhost:8000/soroban/rpc",
                "--network-passphrase",
                "Standalone Network ; February 2017",
            ])
            .assert()
            .success();

        env.set_environments_toml(
            r#"
development.accounts = [
    { name = "alice" },
]

development.network.name = "lol"

[development.contracts]
hello_world.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
        );

        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        assert!(stderr.contains("🌐 using lol network\n"));
    });
}
