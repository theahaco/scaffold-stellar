use std::os::unix::fs::PermissionsExt;
use stellar_scaffold_test::{TestEnv, rpc_url};

/// A minimal shell-script "extension" that echoes each hook name to stdout
/// so we can assert hook firing order and context args
const TEST_EXT_SCRIPT: &str = r#"#!/bin/sh
case "$1" in
  manifest)
    printf '{"name":"test-ext","version":"0.1.0","hooks":["pre-compile","post-compile","pre-deploy","post-deploy","pre-codegen","post-codegen","pre-dev","post-dev"]}'
    ;;
  pre-compile)  echo "test-ext:pre-compile"  ;;
  post-compile) echo "test-ext:post-compile" ;;
  pre-deploy)   echo "test-ext:pre-deploy"   ;;
  post-deploy)  echo "test-ext:post-deploy"  ;;
  pre-codegen)  echo "test-ext:pre-codegen"  ;;
  post-codegen) echo "test-ext:post-codegen" ;;
  pre-dev)      echo "test-ext:pre-dev"      ;;
  post-dev)     echo "test-ext:post-dev"     ;;
esac
"#;

#[test]
fn extension_hooks_fire_in_order() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        // Write the shell-script extension to a temp bin dir and make it executable.
        let bin_dir = env.temp_dir.path().join("ext-bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let script_path = bin_dir.join("stellar-scaffold-test-ext");
        std::fs::write(&script_path, TEST_EXT_SCRIPT).unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Prepend our bin dir so `which stellar-scaffold-test-ext` finds it.
        let custom_path = format!("{}:{}", bin_dir.display(), TestEnv::stellar_path());

        env.set_environments_toml(format!(
            r#"
development.accounts = [
    {{ name = "alice" }},
]
development.extensions = ["test-ext"]

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
            .env("PATH", &custom_path)
            .output()
            .expect("Failed to run scaffold build");

        assert!(
            output.status.success(),
            "Build failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        // pre-dev / post-dev are watch-only hooks and must NOT fire during build.
        assert!(
            !stdout.contains("test-ext:pre-dev"),
            "pre-dev must not fire during build"
        );
        assert!(
            !stdout.contains("test-ext:post-dev"),
            "post-dev must not fire during build"
        );

        // Every build-phase hook should have fired.
        for hook in &[
            "test-ext:pre-compile",
            "test-ext:post-compile",
            "test-ext:pre-deploy",
            "test-ext:post-deploy",
            "test-ext:pre-codegen",
            "test-ext:post-codegen",
        ] {
            assert!(
                stdout.contains(hook),
                "expected hook output not found: {hook}"
            );
        }

        // Verify lifecycle ordering within the output.
        let pos = |marker: &str| {
            stdout
                .find(marker)
                .unwrap_or_else(|| panic!("{marker} not found"))
        };

        assert!(
            pos("test-ext:pre-compile") < pos("test-ext:post-compile"),
            "pre-compile must precede post-compile"
        );
        assert!(
            pos("test-ext:post-compile") < pos("test-ext:pre-deploy"),
            "post-compile must precede pre-deploy"
        );
        assert!(
            pos("test-ext:pre-deploy") < pos("test-ext:post-deploy"),
            "pre-deploy must precede post-deploy"
        );
        assert!(
            pos("test-ext:post-deploy") < pos("test-ext:pre-codegen"),
            "post-deploy must precede pre-codegen"
        );
        assert!(
            pos("test-ext:pre-codegen") < pos("test-ext:post-codegen"),
            "pre-codegen must precede post-codegen"
        );
    });
}
