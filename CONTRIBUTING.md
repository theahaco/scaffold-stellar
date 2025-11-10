# Installation

To install `just`, run the following command:

```bash
cargo install just
```

Make sure cargo-binstall is installed. If not, you can install it with:

```bash
cargo install cargo-binstall
```
https://github.com/cargo-bins/cargo-binstall

Check if you have `cargo-nextest` installed. If not, install it with:

```bash
cargo install cargo-nextest
```
https://crates.io/crates/cargo-nextest

# Setup

To set up the environment, run:

```bash
just setup
```

# Redeploy

To see redeployment in action, use:

```bash
just redeploy
```

# Tests

To run tests, use:

```bash
just test
just test-integration
```

# Troubleshooting

- If you need to clean the project (remove the target folder and all compiled artifacts), run: `cargo clean`.

- When you first open the project in an IDE with Rust Analyzer, it may start building dependencies in the background: `Building compile-time-deps...`. During this process, the `target` folder may be temporarily locked. If you run `just test` or other build commands before this finishes, you may see errors like: `Blocking waiting for file lock on build directory`. Solution: wait for Rust Analyzer’s background build to complete before running commands.

- If you run `just test` or other commands in WSL (Windows Subsystem for Linux), the build may consume a lot of memory. On machines with limited WSL resources, builds can terminate unexpectedly due to out-of-memory errors. Solution: increase WSL resources by editing (or creating) `C:\Users\YOUR_USER\.wslconfig` file if possible.

- For Windows users, please refer [here](./WINDOWS.md).

