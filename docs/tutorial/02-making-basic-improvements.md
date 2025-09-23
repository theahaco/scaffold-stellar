# Making Some Improvements

In our initial version, we had a problem: the `guess` function would crash if no number was set yet. Let's fix this by improving how our contract initializes and by creating reusable code for number generation.

## What We'll Accomplish

By the end of this step, you'll have:

- A contract that sets a number immediately upon deployment
- A private helper function for generating random numbers
- A more robust `reset` function that uses our helper
- Better error handling in the `guess` function

## Understanding the Problem

In our current contract, the `__constructor` only sets the admin, but doesn't set an initial number. This means:

1. If someone calls `guess` before `reset`, it will crash with `unwrap()` on `None`
2. The number generation logic is only in `reset`, making it hard to reuse

Let's fix these issues!

## Step 1: 🔒 Create a Private Helper Function

First, let's extract the number generation into a private helper function. This follows the DRY principle (Don't Repeat Yourself) and makes our code more maintainable.

Open `contracts/guess-the-number/src/lib.rs` and add this private function inside the `impl GuessTheNumber` block:

```rust
#[contractimpl]
impl GuessTheNumber {
    // ... existing functions ...

    /// Private helper function to generate and store a new random number
    fn set_random_number(env: &Env) {
        let new_number: u64 = env.prng().gen_range(1..=10);
        env.storage().instance().set(&THE_NUMBER, &new_number);
    }
}
```

### Understanding Private Functions

Notice that this function doesn't have `pub` in front of it - this makes it private. Private functions:

- Can only be called from within the same contract
- Don't become part of the contract's public API
- Are useful for internal logic and code reuse
- Help keep your contract interface clean and focused

## Step 2: 👷‍♂️ Update the Constructor

Now let's modify the `__constructor` to set an initial number when the contract is deployed:

```rust
pub fn __constructor(env: &Env, admin: &Address) {
    Self::set_admin(env, admin);
    Self::set_random_number(env); // Add this line
}
```

### Why This Improves Things

By setting a number in the constructor:

1. **Immediate functionality**: The contract works right after deployment
2. **No crash risk**: `guess` will never encounter a missing number
3. **Better user experience**: Players can start guessing immediately

## Step 3: ♻️ Update the Reset Function

Let's simplify our `reset` function to use the new helper:

```rust
/// Update the number. Only callable by admin.
pub fn reset(env: &Env) {
    Self::require_admin(env);
    Self::set_random_number(env);
}
```

Much cleaner! The logic is now centralized in our helper function.

## Step 4: ⚠️ Improve Error Handling

Let's make the `guess` function more robust by replacing `unwrap()` with `expect()`:

```rust
/// Guess a number between 1 and 10
pub fn guess(env: &Env, a_number: u64) -> bool {
    let stored_number = env.storage()
        .instance()
        .get::<_, u64>(&THE_NUMBER)
        .expect("No number has been set");

    a_number == stored_number
}
```

### Understanding `expect()` vs `unwrap()`

- `unwrap()`: Crashes with a generic error message
- `expect()`: Crashes with your custom error message
- Both will panic if the value is `None`, but `expect()` gives better debugging info

In our case, this should never happen since we now set a number in the constructor, but it's good defensive programming.

## Step 5: Your Complete Updated Contract

Here's what your `lib.rs` should look like now:

```rust
#![no_std]
use admin_sep::{Administratable, Upgradable};
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

#[contract]
pub struct GuessTheNumber;

#[contractimpl]
impl Administratable for GuessTheNumber {}

#[contractimpl]
impl Upgradable for GuessTheNumber {}

const THE_NUMBER: Symbol = symbol_short!("n");

#[contractimpl]
impl GuessTheNumber {
    pub fn __constructor(env: &Env, admin: &Address) {
        Self::set_admin(env, admin);
        Self::set_random_number(env);
    }

    /// Update the number. Only callable by admin.
    pub fn reset(env: &Env) {
        Self::require_admin(env);
        Self::set_random_number(env);
    }

    /// Guess a number between 1 and 10
    pub fn guess(env: &Env, a_number: u64) -> bool {
        let stored_number = env.storage()
            .instance()
            .get::<_, u64>(&THE_NUMBER)
            .expect("No number has been set");

        a_number == stored_number
    }

    /// Private helper function to generate and store a new random number
    fn set_random_number(env: &Env) {
        let new_number: u64 = env.prng().gen_range(1..=10);
        env.storage().instance().set(&THE_NUMBER, &new_number);
    }
}

mod test;
```

## Step 6: 🧪 Test Your Improvements

Let's test that our improvements work:

### Build and Deploy

```bash
$ stellar contract build

$ stellar contract deploy \
  --wasm target/wasm32v1-none/release/guess_the_number.wasm \
  --source alice \
  --network local
```

### Test Immediate Functionality

Try guessing right after deployment (without calling reset first):

```bash
$ stellar contract invoke \
  --id [CONTRACT_ID] \
  --source alice \
  --network local \
  -- guess --a_number 5
```

This should now work! You'll get either `true` or `false` instead of a crash.

### Test the Reset Function

```bash
$ stellar contract invoke \
  --id [CONTRACT_ID] \
  --source alice \
  --network local \
  -- reset

# Try guessing again
$ stellar contract invoke \
  --id [CONTRACT_ID] \
  --source alice \
  --network local \
  -- guess --a_number 3
```

Perfect! The contract now works reliably from the moment it's deployed.

## What We've Learned

In this step, we covered several important concepts:
1. Code Organization
	- **Private functions**: Help organize code and prevent external access to internal logic
	- **DRY principle**: Don't repeat yourself - extract common logic into reusable functions
2. Contract Lifecycle
	- **Constructor patterns**: Set up initial state when the contract is deployed
	- **Defensive programming**: Always ensure your contract is in a valid state
3. Error Handling
	- **expect() vs unwrap()**: Better error messages help with debugging
	- **Graceful degradation**: Handle edge cases so your contract doesn't crash
4. Blockchain Development Best Practices
	- **Immediate functionality**: Contracts should work right after deployment
	- **Consistent state**: Always maintain valid state throughout the contract's lifecycle

Our contract is now much more robust:

- ✅ Works immediately after deployment
- ✅ Clean, reusable code structure
- ✅ Better error handling
- ❌ Still no authentication (anyone can guess)
- ❌ Still no transactions (how do you win the prize? what prize?)

## What's Next?

In the next step, we'll tackle authentication by:

- Converting `guess` from a view function to a transaction
- Requiring users to be signed in to guess
- Adding a `guesser` parameter to track who made each guess

This will prepare us for adding economic incentives in later steps!
