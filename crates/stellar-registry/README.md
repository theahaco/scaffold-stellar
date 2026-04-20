# stellar-registry

Stellar cross-contract calls simplified.

Say you've got:

1. a contract deployed on Stellar's testnet or mainnet
2. a registered name for this contract in Stellar Registry (example: the `unverified` registry on testnet, which is registered in the official (verified) registry [with the name `unverified`](https://testnet.rgstry.xyz/contracts/unverified))
3. a Wasm hash that is also in Stellar Registry (example: the [`registry`](https://testnet.rgstry.xyz/wasms/registry) Wasm used by the `unverified` contract above)

For now, the `stellar_registry` crate exports one macro: `import_contract_client!`

This macro takes the name of the _Wasm_ binary from Stellar Registry:

```rs
use soroban_sdk; // needs to be in-scope

stellar_registry::import_contract_client!(registry);
```

This creates a `registry` module, equivalent to running:

```bash
stellar registry download registry --out-file target/stellar/registry.wasm
```

...and then importing the Wasm with `soroban_sdk` like:

```rust
mod registry {
    use super::soroban_sdk;
    soroban_sdk::contractimport!(file = "target/stellar/registry.wasm");
}
```

Within a method, you can now instantiate the client as usual, using the contract ID of the desired contract (such as the `unverified` contract above):

```rust
pub fn __constructor(env: &Env, admin: Address) {
    let registry_client = registry::Client::new(
        env,
        &Address::from_str(
            env,
            "CAMLHKQHNZO2IOIBFUF5BGZ2V62BMS5QCWFFGRCB4NOB3G5OMDA7SGZN",
        ),
    );
    let  = registry_client.fetch_contract_id(&String::from_str(env, &"world"));
}
```

# If you don't want your macro making network calls

First, you should know that this macro doesn't make a network call _first_. It starts by looking in the current Cargo project's `target` directory for a `.wasm` file with the given name. Only if it fails to find one will it run `stellar registry download` to download the Wasm before importing it.

If you want to avoid network calls in your build-time macro logic, you can set environment variable `STELLAR_NO_REGISTRY` to `1`.

# More Options

`import_contract_client` is designed to make it easy to paste in Wasm names from https://stellar.rgstry.xyz. If you want to use a channel-prefixed contract or one with hypens in the name, you can use quotes:

```rs
import_contract_client!("unverified/guess-the-number");
```

If you need a specific (historic) version:

```rs
import_contract_client!("registry@v1.0.0");
```

# Future

Eventually, this crate will also export an `import_contract!` macro which will allow importing the _contract_ by name, rather than only the _Wasm_ by name. This will simplify the client creation logic shown above.

Follow progress at https://github.com/theahaco/scaffold-stellar/issues/419
