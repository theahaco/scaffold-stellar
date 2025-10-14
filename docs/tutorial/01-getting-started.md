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


## ✅ Prerequisites

You should have a basic understanding of using the command line and of general programming concepts. Stellar Contracts are written in a subset of Rust, although we'll walk through the code together so don't worry if this is your first time with the language.

- install rust (+ cargo)
- add rust target
- install node (+ npm)
- [install docker](https://docs.docker.com/desktop/) (need to run anything else?)
- install [stellar-cli](https://github.com/stellar/stellar-cli) and [scaffold stellar](https://github.com/theahaco/scaffold-stellar) plugin

TODO: rewrite our own?
[Follow original setup instructions.](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup)


## 🏗️ Create the Scaffold

Our smart contract will be a Guess The Number game. You (the admin) can deploy the contract, randomly select a number between 1 and 10, and seed the contract with a prize. Users can make guesses and win the prize if they're correct!

> ℹ️ Why a guessing game? A standard Hello World program isn't all that useful of an example. Unlike most smart contracts, there's nothing to interact with. That's why the Rust Programming Language book uses a guessing game as [their first tutorial project](https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html). We thought we'd do the same.

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

The `contracts` directory holds each smart contract as a separate package in our project's workspace. We only need the one for this project, but it's nice to know that we can use the same structure for more complex projects that require multiple contracts. The other example contracts in this folder come from our friends at [OpenZeppelin](https://wizard.openzeppelin.com/stellar).

We can configure how and where our contract will be built and deployed to in the `environments.toml` file. And if we generate client code from our contract, that will go in the `packages/` directory which is currently empty (because we haven't built anything yet!). We'll do that soon.

Finally, the `rust-toolchain.toml` file notes which version of Rust we're using and what platform we're targeting. We won't worry about the rest of the files in here yet, they're all for the frontend dApp we'll talk about later.

## 🔎 Understand the Starter Code

Let's open up the initial smart contract code in `contracts/guess-the-number/src/lib.rs` and walk through it.

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol};
```

Rust has a great [standard library](https://doc.rust-lang.org/std/) of types, functions, and other abstractions. But our smart contract will run in a constrained WebAssembly environment where the full library isn't available or needed. The `#![no_std]` directive forces us to use only core Rust features.

We can still use explicitly imported features, though, and that's what the next line is doing. Here we're importing some essential items from Stellar's Soroban SDK, and we'll explain each as we get to them. You'll see that many of them replace items from the standard library but are designed for use in Soroban's environment. And the first is `contract`:

```rust
#[contract]
pub struct GuessTheNumber;
```

The `#[...]` syntax in Rust is called an [attribute](https://doc.rust-lang.org/reference/attributes.html). It's a way to label code for the compiler to handle it with special instructions. *Inner* attributes (with the `#!`) apply to the scope they're within, and *Outer* attributes (just the `#`) apply to the next line.

Here defining a [struct](https://doc.rust-lang.org/book/ch05-01-defining-structs.html) (a "structure" to hold values) and applying attributes of a Stellar smart contract. Then we'll use the pre-defined [traits](https://doc.rust-lang.org/book/ch10-02-traits.html) we just explained to add more functionality to our contract without actually having to write it ourselves.

```rust
const THE_NUMBER: Symbol = symbol_short!("n");
pub const ADMIN_KEY: &Symbol = &symbol_short!("ADMIN");
```

Now the most important part of our contract: the number! This line creates a key for storing and retrieving contract data. A `Symbol` is a short string type (max 32 characters) that is more optimized for use on the blockchain. And we're using the `symbol_short` macro for an even smaller key (max 9 characters). As a contract author, you want to use tricks like this to lower costs as much as you can.

The second line creates a key for storing the address of this contract's administrator. It's almost the same code as storing our number, but uses the `&` which is called a [reference](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html). Instead of the value, it's a pointer to where the value lives.

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


## 👷‍♀️ Let's Build and Test It Locally

Now that we have some contract code in Rust, we can deploy it and try interacting with it. Eventually we'll run this contract on Stellar's Mainnet, the production level network with real users and financial connections. But we should test our code first. Stellar maintains a Testnet, a free-to-use network that mimics a production environment. That's a perfect place to test with real network conditions and for sharing test code with others, and we'll do this later. But there's an even quicker option: running a local network.

> ℹ️ [Read more about the differences between networks.](https://developers.stellar.org/docs/learn/fundamentals/networks)

Stellar CLI has a built-in command to easily configure and manage a Docker container that will run the local network for you. Make sure Docker is running in the background and run the following command in your terminal:

```bash
$ stellar container start local
  ℹ️ Starting local network
  ...
  ✅ Started container
```

This will download the image and start up the container as a mini Stellar network ready for us to use. Now let's build the contract. That means compiling the Rust code into WebAssembly (a `.wasm` file that can run on the network) as well as [creating the interface types](https://developers.stellar.org/docs/learn/fundamentals/contract-development/types/fully-typed-contracts) (a specification that can be used by other contracts or code to interact with your contract). 

```bash
$ stellar contract build
```

Sharp eyed readers might notice these lines in the build output:

```
  Exported Functions: 7 found
    • _
    • __constructor
    • admin
    • guess
    • reset
    • set_admin
    • upgrade
✅ Build Complete
```

We implemented a few of those functions in our contract (i.e. `__constructor`, `guess`, and `reset`), but where did the others come from? They are inherited from the `Administratable` and `Ugradable` traits we talked about earlier.

## 🚢 Deploy To Local Network

In order to deploy the contract, we need an account on the local network to own the contract and sign transactions. Let's do that now and also provide some [fake funds](https://developers.stellar.org/docs/learn/fundamentals/networks#friendbot) to test with.

```bash
$ stellar keys generate alice --network local --fund
```

You can use any name you want to, but it's nice to have a few identities created for use in testing. `Alice` will be our contract administrator. We'll create more identities later to test other interactions.

Time to deploy!

```bash
$ stellar contract deploy \
  --wasm target/wasm32v1-none/release/guess_the_number.wasm \
  --alias guess_the_number \
  --network local \
  --source-account alice \
  -- --admin alice
```

Let's break this long command down by each flag:
  - `--wasm`: the path to the `.wasm` file we just built from our contract
  - `--alias`: a human-friendly name for our contract instead of working with a long hashed identifier
  - `--network`: the network we're targeting for deployment
  - `--source-account`: the account that's funding the deployment (Note: this can be different than the admin)
  - `--`: anything after these hyphens are methods and arguments provided to the contract. Since we don't list a method, we'll use the `__constructor` and we only have one argument:
    - `--admin`: the account that can administrate the contract
    
After deployment, our `__constructor` method is run which initializes our contract. This is a perfect time to setup anything your contract needs to run. And for contracts with a lot of constructor arguments, you can also define them inside the `environments.toml` file:

```toml
[development.contracts.guess_the_number]
# ...
constructor_args = """
--admin me
"""
```

In fact, there's a lot of other configuration you can do for each deployment inside the `environments.toml` file, so we're going to rely on that from now on. You'll notice it's broken down into sections, one for each environment you might be deploying to. We'll specify which to use based on an environment variable. We already set one for you in the `.env` file and defaulted to use the `development` environment:

 ```bash
 # The environment to use `development`, `testing`, `staging`, `production`
 STELLAR_SCAFFOLD_ENV=development
 ```

I mentioned that we're using an alias to refer to our deployed contract instead of its hash. You can view those aliases with:

```bash
$ stellar contract alias ls
```


## 🏃 Run The Contract

We can use the Stellar CLI to run some of our contract methods and test them out. We'll target the contract to run using the alias we just created and the same network and source account as before. But this time we'll specify one of our functions to run and provide an argument:

```bash
$ stellar contract invoke \
  --id guess_the_number \
  --network local \
  --source-account alice \
  -- guess --a_number 1
```

💥 Error!

What happened? Well, we tried to guess a number that doesn't exist yet! Let's do that first:

```bash
$ stellar contract invoke \
  --id guess_the_number \
  --network local \
  --source-account alice \
  -- reset
```

Now we have a number stored on the network to actually guess against. Try running the guess method again. You should see `true` or `false`. Keep guessing until you get it right 🙂.

> ℹ️ There is a handy way to explore contract methods and arguments directly from the Stellar CLI. Run the above invoke command but replace the `reset` method with `help`. You should see a list of all the available methods on the contract. You can inspect an individual method's documentation with `help <method>`, like `help reset`.

## 🤔 Is That It?

Technically, yeah! That's all you need to create a contract and deploy it on chain. But it isn't exactly the easiest to interact with, is it?

Good thing there's more parts to this tutorial! We're going to create a front-end application to interact with this contract code so we can get others to use the contract in a human-friendly way.

## 😲 The dApp

Let's run the front-end application and see a better way to interact with the contract. Open your terminal, we'll install the UI's dependencies with npm and then start the app:

```bash
$ npm install
$ npm start
```

In the output, you'll see a few things happening. The Stellar CLI is building the client code for all our contracts in the project (the output starting with [0]) and we're also starting up the React app using Vite (the output starting with [1]). Both processes are happening at the same time, and we'll talk about why in the next step.

But for now, you should see a line telling you that Vite is running and you can visit [http://localhost:5174/]() in your browser. Open it up!

### What Am I Looking At?

On the surface, it's a standard React application. But we're doing some nifty things under the hood. The Stellar CLI client code I mentioned is generated TypeScript based on our contract. You'll find it in the `packages/` directory if you're curious to poke around at it. It lists out all the methods on all your contracts, including the types of their arguments and return values. That way we can safely call methods on the contract from the client and know what we'll be working with.

We also generate an RPC client to actually do that for you! Each of those clients, one for each contract, is in the `src/contracts` directory. We'll dive deeper into how this all works in the next step. The basic gist is that all the hard boring stuff is done for you!

Wait, really? Yes. Really. So as you update your contract code, you can jump right over to a React component and be able to call methods from it.

### Try It Out!

In the top right corner, you'll see a big "Connect" button. Click it. You need to have a [Wallet](https://stellar.org/learn/wallets-to-store-send-and-receive-lumens) in order to interact with the dApp. The modal that opened will show a few options if you don't have one already. We recommend using [Freighter](https://www.freighter.app/). 

Once it's installed, we need to connect it to our local network running in Docker. Open the extension, click the menu, and navigate to "Settings," then "Network." Click the "Add custom network" button and enter the following info:

- **Name**: `Local`
- **HORIZON RPC URL**: `http:localhost:8000`
- **SOROBAN RPC URL**: `http:localhost:8000/rpc`
- **Passphrase**: `Standalone Network ; February 2017`
- **Friendbot URL**: `http:localhost:8000/friendbot`
- Check **Allow connecting to non-HTTPS networks**

> ℹ️ The 🌐 icon in the extension lets you switch back and forth between this Local network as well as test and main net.

Now click the dApp's "Connect" button and follow the prompts to let the application communicate with Freighter. If it's successful, you should see your account info in the header along with a new "Fund Account" button and a tag for the current network. On the homepage, you should also see a component to test out the Guess the Number contract.

Go ahead and enter some guesses. Right out of the gate we have nice UI to invoke methods on the contract

## Summary

That covered a lot, but let's list out how simple it actually was:

1. We ran `stellar scaffold init my-project` to generate example clients and a whole UI for them
2. We started up a local Stellar network and created identities, but we only have to do that once
3. We ran `npm start` to deploy the contracts to our local network and run the application

That's it! Scaffold Stellar does all the heavy lifting letting you jump right in to the fun parts of developing your contract and applications. 🎉

## What We've Learned

In this step, we covered several important concepts:
1. Project Structure
    - **Scaffold**: Bootstrap new projects using `stellar scaffold init`
    - **Organization**: Manage contract code, dApp code, and configuration in one place
2. Smart Contract Basics
    - **Rust**: A subset of the language targeting Stellar's WebAssembly environment
    - **Storage**: Retrieving data stored on the contract
3. Local Development
    - **Network**: Using Docker to run a local Stellar network to speed development and testing
    - **Identities**: Creating accounts for testing by generating keys
    - **Aliases**: Using friendly names instead of long hash identifiers
4. Contract Lifecycle:
    - **Deploying**: Publishing the contract on-chain
    - **Invoking**: Running contract methods

That's a good start, but there's a lot left to do:

- ✅ Works immediately after deployment
- ✅ Clean, reusable code structure
- ✅ Interact with the contract via the dApp
- ❌ Learn how to use the Contract Explorer
- ❌ Fix the bug that caused an error when guessing
- ❌ Requires manual testing

## What's Next?

Scaffold Stellar actually took care of that for us! In the next step, we'll make some improvements by:

- Rewriting some contract code to make it more robust
- Running the React UI in dev mode to see live updates from code changes
- Debugging methods via the Contract Explorer
- Using tests to make sure our contract code is sound

That will give you a better sense of the typical development workflow for contracts and dApps.
