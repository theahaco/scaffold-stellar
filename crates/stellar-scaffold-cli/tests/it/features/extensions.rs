use std::os::unix::fs::PermissionsExt;
use stellar_scaffold_test::{TestEnv, rpc_url};

/// A minimal shell-script "extension" that echoes each hook name to stdout
/// so we can assert hook firing order and context args
const TEST_EXT_SCRIPT: &str = r#"#!/bin/sh
case "$1" in
  manifest)
    printf '{"name":"test-ext","version":"0.1.0","hooks":["pre-compile","post-compile","pre-deploy","post-deploy","pre-codegen","post-codegen","pre-dev","post-dev"]}'
    exit 0
    ;;
esac
# Hook invocations receive a JSON context on stdin; consume it so the
# parent's write_all completes without a broken-pipe error.
cat > /dev/null
case "$1" in
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
            .env("RUST_LOG", "warn")
            .output()
            .expect("Failed to run scaffold build");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "Build failed.\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
        );

        // pre-dev / post-dev are watch-only hooks and must NOT fire during build.
        assert!(
            !stdout.contains("test-ext:pre-dev"),
            "pre-dev must not fire during build.\nSTDOUT:\n{stdout}"
        );
        assert!(
            !stdout.contains("test-ext:post-dev"),
            "post-dev must not fire during build.\nSTDOUT:\n{stdout}"
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
                "expected hook output not found: {hook}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
            );
        }

        // Verify lifecycle ordering within the output.
        let pos = |marker: &str| {
            stdout
                .find(marker)
                .unwrap_or_else(|| panic!("{marker} not found in stdout:\n{stdout}"))
        };

        assert!(
            pos("test-ext:pre-compile") < pos("test-ext:post-compile"),
            "pre-compile must precede post-compile.\nSTDOUT:\n{stdout}"
        );
        assert!(
            pos("test-ext:post-compile") < pos("test-ext:pre-deploy"),
            "post-compile must precede pre-deploy.\nSTDOUT:\n{stdout}"
        );
        assert!(
            pos("test-ext:pre-deploy") < pos("test-ext:post-deploy"),
            "pre-deploy must precede post-deploy.\nSTDOUT:\n{stdout}"
        );
        assert!(
            pos("test-ext:post-deploy") < pos("test-ext:pre-codegen"),
            "post-deploy must precede pre-codegen.\nSTDOUT:\n{stdout}"
        );
        assert!(
            pos("test-ext:pre-codegen") < pos("test-ext:post-codegen"),
            "pre-codegen must precede post-codegen.\nSTDOUT:\n{stdout}"
        );
    });
}

/// Regression test for a past CI failure where `post-codegen` was silently
/// skipped whenever any step inside `generate_contract_bindings` errored
/// (the `?` short-circuits returned before the hook fired). The lifecycle
/// contract is: every fired pre-X has a matching post-X, regardless of
/// whether the inner step succeeded.
#[test]
fn extension_post_codegen_fires_when_codegen_step_fails() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        let bin_dir = env.temp_dir.path().join("ext-bin");
        std::fs::create_dir_all(&bin_dir).unwrap();

        // Same test extension as above.
        let script_path = bin_dir.join("stellar-scaffold-test-ext");
        std::fs::write(&script_path, TEST_EXT_SCRIPT).unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Shim `npm` to exit non-zero. Codegen's `npm install` / `npm run build`
        // steps inside generate_contract_bindings will fail deterministically.
        let npm_shim = bin_dir.join("npm");
        std::fs::write(
            &npm_shim,
            "#!/bin/sh\necho 'npm shim: forcing failure' >&2\nexit 99\n",
        )
        .unwrap();
        std::fs::set_permissions(&npm_shim, std::fs::Permissions::from_mode(0o755)).unwrap();

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
            .env("RUST_LOG", "warn")
            .output()
            .expect("Failed to run scaffold build");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // pre-codegen must fire (it runs before the rebuild block) and
        // post-codegen must still fire even though the rebuild block errored.
        assert!(
            stdout.contains("test-ext:pre-codegen"),
            "pre-codegen must fire before the rebuild block.\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
        );
        assert!(
            stdout.contains("test-ext:post-codegen"),
            "post-codegen must fire even when an inner codegen step errored — this is the lifecycle contract.\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
        );
    });
}
