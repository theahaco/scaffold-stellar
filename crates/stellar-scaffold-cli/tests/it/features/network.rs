use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};

#[test]
fn run_network_from_rpc_and_passphrase() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(format!(
            r#"
development.accounts = [
    {{ name = "alice" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url()
        ));

        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        assert!(stderr.contains(&format!("Using network at {}\n", rpc_url())));
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
                rpc_url().as_str(),
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
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
        );

        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        assert!(stderr.contains(&format!("Using network at {}\n", rpc_url())));
    });
}
