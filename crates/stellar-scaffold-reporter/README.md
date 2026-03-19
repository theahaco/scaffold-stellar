# Scaffold Reporter

An extension for [Scaffold Stellar](https://scaffoldstellar.org) to measure and log useful metrics for contract and dApp developers.

## 📐 What Metrics?

- **WASM size:** When you deploy a Stellar smart contract, you're uploading a compiled binary (the `.wasm` file) to a shared global ledger. _Every byte you store costs fees, and there's a hard cap of ~128KB._ As your contract grows, you'll start paying unexpectedly more or even hit the cap. Treat it like bundle size in a web app.
- **WASM hash:** Think of this as a git commit hash for your compiled binary. _Two identical compilations produce the same hash and the network uses this to deduplicate._ If that hash is already uploaded, it skips the upload and just creates a new instance pointing to the existing code.
- **Deploy vs. Upgrade:** Fresh deploys create a brand-new contract with a new address. Upgrades swap the code at the existing address _while preserving all its stored data._ Think of this like a schema migration for an existing database. The environment matters. Fresh deploys during development are fine, but accidental deploys in production mean your frontend is pointing at a dead address.
- **Compile duration:** Stellar contracts compile to WebAssembly. _Rust → WASM compilation can be slow._ Tracking it over time catches CI regressions.
- **TypeScript package size:** Scaffold generates a TypeScript client bundled with your frontend. _A large bundle has a real cost to users downloading your dApp._
- **Total build time:** This is the end-to-end latency of `stellar scaffold watch`, from "I saved a file" to "my frontend client is regenerated." _This is your core development feedback loop._

## 📦 Installation

If you started your project with `stellar scaffold init`, congrats! You already have it!

Otherwise, install the crate with [Cargo]():

``` sh
cargo install stellar-scaffold-reporter
```

And register it in your Scaffold `environments.toml` file:

``` toml
[development]
extensions = [reporter]
```

## 🏃 Running the Reporter

That's all you need! Any time you run a `stellar scaffold build` or `stellar scaffold watch` command, you'll see the reports logged to the console. You can also view reports in the `.scaffold/reports/` directory.

## ⚙️ Configuration

TODO
