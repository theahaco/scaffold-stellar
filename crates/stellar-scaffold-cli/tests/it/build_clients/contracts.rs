use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};

#[test]
fn contracts_built() {
    let contracts = [
        "soroban_auth_contract",
        "soroban_custom_types_contract",
        "soroban_hello_world_contract",
        "soroban_increment_contract",
    ];
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(
            format!(
                r#"
development.accounts = [
    {{ name = "alice" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_token_contract.client = false
{}
"#,
                rpc_url(),
                contracts
                    .iter()
                    .map(|c| format!("{c}.client = true"))
                    .collect::<Vec<String>>()
                    .join("\n")
            )
            .as_str(),
        );

        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        assert!(stderr.contains("Creating keys for \"alice\"\n"));
        assert!(stderr.contains(format!("Using network at {}\n", rpc_url()).as_str()));

        for c in contracts {
            assert!(stderr.contains(&format!("Uploading \"{c}\" wasm bytecode on-chain")));
            assert!(stderr.contains(&format!("Instantiating \"{c}\" smart contract")));
            assert!(stderr.contains(&format!("Binding \"{c}\" contract")));

            // check that contracts are actually deployed, bound, and imported
            assert!(env.cwd.join(format!("packages/{c}")).exists());
            assert!(env.cwd.join(format!("src/contracts/{c}.ts")).exists());

            // check dist/index.js and dist/index.d.ts exist after npm run build
            let dist_dir = env.cwd.join(format!("packages/{c}/dist"));
            assert!(
                dist_dir.join("index.js").exists(),
                "index.js missing for {c}"
            );
            assert!(
                dist_dir.join("index.d.ts").exists(),
                "index.d.ts missing for {c}"
            );
        }
    });
}

#[test]
fn contracts_built_by_default() {
    let contracts = [
        "soroban_auth_contract",
        "soroban_custom_types_contract",
        "soroban_hello_world_contract",
        "soroban_increment_contract",
    ];
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
soroban_token_contract.client = false

"#,
            rpc_url()
        ));
        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        println!("{stderr}");
        assert!(stderr.contains("Creating keys for \"alice\"\n"));
        // assert!(stderr.contains(&format!("Using network at {}\n", rpc_url())));

        for c in contracts {
            assert!(stderr.contains(&format!("Uploading \"{c}\" wasm bytecode on-chain")));
            assert!(stderr.contains(&format!("Instantiating \"{c}\" smart contract")));
            assert!(stderr.contains(&format!("Binding \"{c}\" contract")));

            // check that contracts are actually deployed, bound, and imported
            assert!(env.cwd.join(format!("packages/{c}")).exists());
            assert!(env.cwd.join(format!("src/contracts/{c}.ts")).exists());
        }
    });
}

#[test]
fn contract_with_bad_name_prints_useful_error() {
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
hello.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url()
        ));

        env.scaffold("build")
            .assert()
            .failure()
            .stderr(predicates::str::contains("No contract named \"hello\""));
    });
}

#[test]
fn contract_alias_skips_install() {
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
soroban_hello_world_contract.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url()
        ));

        let output = env
            .scaffold_build("development", false)
            .output()
            .expect("Failed to execute command");

        // ensure it imports
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("Binding \"soroban_hello_world_contract\" contract")
        );

        let output2 = env
            .scaffold_build("development", false)
            .output()
            .expect("Failed to execute command");

        // ensure alias retrieval works
        eprintln!("{:?}", String::from_utf8_lossy(&output2.stderr));
        assert!(output2.status.success());
        assert!(
            String::from_utf8_lossy(&output2.stderr)
                .contains("Contract \"soroban_hello_world_contract\" is up to date")
        );

        let output3 = env
            .scaffold_build("development", true)
            .output()
            .expect("Failed to execute command");

        // ensure contract hash change check works, should update in dev mode
        assert!(output3.status.success());
        let message = String::from_utf8_lossy(&output3.stderr);
        assert!(message.contains("Updating contract \"soroban_hello_world_contract\""));
        let Some(contract_id) = extract_contract_id(&message) else {
            panic!("Could not find contract ID in stderr");
        };
        env.set_environments_toml(format!(
            r#"
production.accounts = [
    {{ name = "alice" }},
]

[production.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[production.contracts]
soroban_hello_world_contract.id = "{contract_id}"
"#,
            rpc_url()
        ));

        // ensure production can identify via contract ID
        env.scaffold_build("production", true).assert().success();

        env.set_environments_toml(format!(
            r#"
production.accounts = [
    {{ name = "alice" }},
]

[production.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[production.contracts]
soroban_hello_world_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url()
        ));

        let output4 = env
            .scaffold_build("production", true)
            .output()
            .expect("Failed to execute command");

        // ensure contract hash change check works, should throw error in production
        assert!(!output4.status.success());
        assert!(
            String::from_utf8_lossy(&output4.stderr)
                .contains("️An ID must be set for a contract in production or staging")
        );
    });
}

fn extract_contract_id(stderr: &str) -> Option<String> {
    stderr
        .lines()
        .find(|line| line.contains("contract_id:"))
        .and_then(|line| {
            line.split_whitespace()
                .last()
                .map(|id| id.trim().to_string())
        })
}

#[test]
fn contract_redeployed_in_new_directory() {
    let mut env = TestEnv::new("soroban-init-boilerplate");

    // Initial setup and build
    env.set_environments_toml(format!(
        r#"
development.accounts = [
    {{ name = "alice" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
        rpc_url()
    ));

    let output = env
        .scaffold_build("development", true)
        .output()
        .expect("Failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stderr}");
    assert!(stderr.contains("Uploading \"soroban_hello_world_contract\" wasm bytecode on-chain"));
    assert!(stderr.contains("Instantiating \"soroban_hello_world_contract\" smart contract"));
    assert!(stderr.contains("Simulating deploy transaction"));
    assert!(stderr.contains("Binding \"soroban_hello_world_contract\" contract"));

    // Switch to a new directory

    env.switch_to_new_directory("soroban-init-boilerplate", "new-dir")
        .expect("should copy files and switch to new dir");
    // Set up the new directory with the same configuration
    env.set_environments_toml(format!(
        r#"
development.accounts = [
    {{ name = "alice" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
        rpc_url()
    ));

    // Run build in the new directory
    let output = env
        .scaffold_build("development", true)
        .output()
        .expect("Failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stderr}");
    assert!(stderr.contains("Uploading \"soroban_hello_world_contract\" wasm bytecode on-chain"));
    assert!(stderr.contains("Instantiating \"soroban_hello_world_contract\" smart contract"));
    assert!(stderr.contains("Simulating deploy transaction"));
    assert!(stderr.contains("Binding \"soroban_hello_world_contract\" contract"));
    // Check that the contract files are created in the new directory
    assert!(
        env.cwd
            .join("packages/soroban_hello_world_contract")
            .exists()
    );
    assert!(
        env.cwd
            .join("src/contracts/soroban_hello_world_contract.ts")
            .exists()
    );
}

#[test]
fn contract_built_with_out_dir() {
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
soroban_hello_world_contract.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url()
        ));

        let out_dir = env.cwd.join("custom_wasm_output");

        let stderr = env
            .scaffold("build")
            .arg("--out-dir")
            .arg(&out_dir)
            .assert()
            .success()
            .stderr_as_str();

        assert!(stderr.contains("Creating keys for \"alice\"\n"));
        assert!(stderr.contains(&format!("Using network at {}\n", rpc_url())));
        assert!(
            stderr.contains("Uploading \"soroban_hello_world_contract\" wasm bytecode on-chain")
        );
        assert!(stderr.contains("Instantiating \"soroban_hello_world_contract\" smart contract"));
        assert!(stderr.contains("Binding \"soroban_hello_world_contract\" contract"));

        // Check that WASM file was copied to custom out_dir
        assert!(out_dir.join("soroban_hello_world_contract.wasm").exists());

        // Check that contract client files are still generated
        assert!(
            env.cwd
                .join("packages/soroban_hello_world_contract")
                .exists()
        );
        assert!(
            env.cwd
                .join("src/contracts/soroban_hello_world_contract.ts")
                .exists()
        );

        // Check dist/index.js and dist/index.d.ts exist after npm run build
        let dist_dir = env.cwd.join("packages/soroban_hello_world_contract/dist");
        assert!(
            dist_dir.join("index.js").exists(),
            "index.js missing for soroban_hello_world_contract"
        );
        assert!(
            dist_dir.join("index.d.ts").exists(),
            "index.d.ts missing for soroban_hello_world_contract"
        );
    });
}

#[test]
fn contracts_with_failures_show_summary() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        // First pass - build wasm and create accounts
        env.set_environments_toml(format!(
            r#"
development.accounts = [
    {{ name = "alice" }},
    {{ name = "bob" }},
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

        // First build to generate accounts and wasm
        env.scaffold("build").assert().success();

        // Second pass - try to build clients with incorrect constructor args
        env.set_environments_toml(format!(
            r#"
development.accounts = [
    {{ name = "alice" }},
    {{ name = "bob" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = true
soroban_custom_types_contract.client = true
soroban_auth_contract.client = false

[development.contracts.soroban_token_contract]
client = true
constructor_args = """
STELLAR_ACCOUNT=bob --symbol ABND --decimal 7 --name abundance --admin bb 
"""
"#,
            rpc_url()
        ));

        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        eprintln!("{stderr}");

        // Should show summary of results
        assert!(stderr.contains("Client Generation Summary:"));
        assert!(stderr.contains("Successfully processed: 3"));
        assert!(stderr.contains("Failed: 1"));
        assert!(stderr.contains("Failures:"));

        // Should show specific failure details for token contract
        assert!(stderr.contains("soroban_token_contract:"));

        // Should still process successful contracts
        assert!(
            stderr.contains("Uploading \"soroban_hello_world_contract\" wasm bytecode on-chain")
        );
        assert!(stderr.contains("Uploading \"soroban_increment_contract\" wasm bytecode on-chain"));
        assert!(
            stderr.contains("Uploading \"soroban_custom_types_contract\" wasm bytecode on-chain")
        );

        // Check that successful contracts are still deployed
        assert!(
            env.cwd
                .join("packages/soroban_hello_world_contract")
                .exists()
        );
        assert!(env.cwd.join("packages/soroban_increment_contract").exists());
        assert!(
            env.cwd
                .join("packages/soroban_custom_types_contract")
                .exists()
        );
        assert!(
            env.cwd
                .join("src/contracts/soroban_hello_world_contract.ts")
                .exists()
        );
        assert!(
            env.cwd
                .join("src/contracts/soroban_increment_contract.ts")
                .exists()
        );
        assert!(
            env.cwd
                .join("src/contracts/soroban_custom_types_contract.ts")
                .exists()
        );

        // Failed contract should not have generated client files
        assert!(!env.cwd.join("packages/soroban_token_contract").exists());
        assert!(
            !env.cwd
                .join("src/contracts/soroban_token_contract.ts")
                .exists()
        );
    });
}
