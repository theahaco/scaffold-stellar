# Getting Started with Scaffold Stellar

We'll use Scaffold Stellar to create a new Smart Contract project and walk through some simple improvements to show common blockchain interactions that will be helpful when you start crafting your own contracts. This includes:

- storing data
- authentication
- handling transactions
- obfuscating private data

While building these features, you'll also learn the life cycle of smart contract development including

- compiling
- debugging
- testing
- deploying
- and upgrading


## Prerequisites

You should have a basic understanding of using the command line and of general programming concepts. Stellar Contracts are written in a subset of Rust, although we'll walk through the code together so don't worry if this is your first time with the language.

- install rust
- add rust target
- install scaffold stellar


# 🏗️ Create the Scaffold

Our smart contract will be a Guess The Number game. You (the admin) can deploy the contract, randomly select a number between 1 and 10, and seed the contract with a prize. Users can make guesses and win the prize if they're correct!

Let's use the Stellar CLI tool to get a starting point. Open your terminal and navigate to the directory you keep your projects, then type:

```bash
$ stellar scaffold init my-project
```


You can call your project anything you'd like. Navigate into the created directory and you will see a generated project structure including many of these files and folders:

```
.
├── Cargo.lock
├── Cargo.toml
├── contracts/
│   ├── guess_the_number/
│   │   ├── Cargo.toml
│   │   ├── Makefile
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── test.rs
├── environments.toml
├── packages/
├── README.md
└── rust-toolchain.toml
```

The `Cargo.toml` file is called the project's [manifest](https://doc.rust-lang.org/cargo/reference/manifest.html) and it contains metadata needed to compile everything and package it up. It's where you can name and version your project as well as list the dependencies you need. But we'll look at this later.

`Cargo.lock` is Rust's [lockfile](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html) and it has exact info about the dependencies we actually installed. It's maintained by Cargo and we shouldn't touch it.

The `contracts` directory holds each smart contract as a separate package in our project's workspace. We only need the one for this project, but it's nice to know that we can use the same structure for more complex projects that require multiple contracts.

We can configure how and where our contract will be built and deployed to in the `environments.toml` file. And if we generate client code from our contract, that will go in the `packages/` directory which is currently empty (because we haven't built anything yet!). We'll do that soon.

Finally, the `rust-toolchain.toml` file notes which version of Rust we're using and what platform we're targeting. We won't worry about the rest of the files in here yet, they're all for the frontend dApp we'll talk about later.

## 🔎 Understand the Starter Code

Let's open up the initial smart contract code in `contracts/guess-the-number/src/lib.rs` and walk through it.

```rust
#![no_std]
use admin_sep::{Administratable, Upgradable};
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};
```

Rust has a great [standard library](https://doc.rust-lang.org/std/) of types, functions, and other abstractions. But our smart contract will run in a constrained WebAssembly environment where the full library isn't available or needed. The `#![no_std]` directive forces us to use only core Rust features.

We can still use explicitly imported features, though, and that's what the next two lines are doing. Here we're importing two traits from the `admin_sep` crate to help with admin functionality:
- `Administratable` provides functions like `require_admin()` to manage who can preform administrative actions
- `Upgradable` allows the contract to be upgraded to new versions while preserving its state

We're also importing essential items from Stellar's Soroban SDK, and we'll explain each as we get to them. You'll see that many of them replace items from the standard library but are designed for use in Soroban's environment. And the first is `contract`:

```rust
#[contract]
pub struct GuessTheNumber;

#[contractimpl]
impl Administratable for GuessTheNumber {}

#[contractimpl]
impl Upgradable for GuessTheNumber {}
```

The `#[...]` syntax in Rust is called an [attribute](https://doc.rust-lang.org/reference/attributes.html). It's a way to label code for the compiler to handle it with special instructions. *Inner* attributes (with the `#!`) apply to the scope they're within, and *Outer* attributes (just the `#`) apply to the next line.

Here defining a [struct](https://doc.rust-lang.org/book/ch05-01-defining-structs.html) (a "structure" to hold values) and applying attributes of a Stellar smart contract. Then we'll use the pre-defined [traits](https://doc.rust-lang.org/book/ch10-02-traits.html) we just explained to add more functionality to our contract without actually having to write it ourselves.

```rust
const THE_NUMBER: Symbol = symbol_short!("n");
```

Now the most important part of our contract: the number! This line creates a key for storing and retrieving contract data. A `Symbol` is a short string type (max 32 characters) that is more optimized for use on the blockchain. And we're using the `symbol_short` macro for an even smaller key (max 9 characters). As a contract author, you want to use tricks like this to lower costs as much as you can.

```rust
#[contractimpl]
impl GuessTheNumber {
```

Let's `impl`ement our contract's functionality.

```rust
    pub fn __constructor(env: &Env, admin: &Address) {
        Self::set_admin(env, admin);
    }
```

A contract's `constructor` runs when it is deployed. In this case, we're saying who has access to the admin functions. We don't want anyone to be able to reset our number, do we?!

```rust
    /// Update the number. Only callable by admin.
    pub fn reset(env: &Env) {
        Self::require_admin(env);
        let new_number: u64 = env.prng().gen_range(1..=10);
        env.storage().instance().set(&THE_NUMBER, &new_number);
    }
```

And here is the reset function. Note that we use `require_admin()` here so only you can run this function. It generates a random number between 1 and 10 and uses our key to store it.

```rust
    /// Guess a number between 1 and 10
    pub fn guess(env: &Env, a_number: u64) -> bool {
        a_number == env.storage().instance().get::<_, u64>(&THE_NUMBER).unwrap()
    }
}
```

Finally, we add the `guess` function which accepts a number as the guess and compares it to the stored number, returning the result. Notice we're using our defined key (that small Symbol) to find stored data that may or may not be there. That's why we need [`unwrap()`](https://doc.rust-lang.org/rust-by-example/error/option_unwrap.html), but we'll talk more about `Option` values later in the tutorial.

```rust
mod test;
```

Post Script: this last line includes the test module into this file. It's handy to write unit tests for our code in a separate file (`contracts/guess-the-number/src/test.rs`), but you could also write them inline if you want.
