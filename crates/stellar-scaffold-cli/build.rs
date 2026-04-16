fn main() {
    crate_git_revision::init();

    // cargo_bin!("stellar-scaffold-reporter") in integration tests expands to
    // env!("CARGO_BIN_EXE_stellar-scaffold-reporter"), which Cargo sets for
    // same-package binaries and dev-dependency binaries during `cargo test` but
    // NOT during `cargo clippy --tests`.  Emitting it here ensures it is always
    // present at compile time regardless of how the crate is being built.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    // OUT_DIR = target/<profile>/build/<hash>/out — 3 levels up is target/<profile>/
    let target_dir = std::path::Path::new(&out_dir).ancestors().nth(3).unwrap();
    let exe_suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    println!(
        "cargo:rustc-env=CARGO_BIN_EXE_stellar-scaffold-reporter={}",
        target_dir
            .join(format!("stellar-scaffold-reporter{exe_suffix}"))
            .display()
    );
}
