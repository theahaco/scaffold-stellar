# Adding in Transactions

Now comes the exciting part: adding some economic incentives to our guessing game! We'll implement a system where users pay to play and winners take the entire pot. This is where blockchain development gets really interesting. And honestly, it's probably why you're here in the first place, right?

## What We'll Accomplish

By the end of this step, you'll have:

- A guessing fee that players must pay
- A prize pot that accumulates all the fees
- Automatic winner payouts
- Admin funding of the initial prize pot
- Understanding of token transfers in smart contracts

## Understanding the Economic Model

Here's how our game economics will work:

1. **Admin funds the pot**: When resetting, admin transfers XLM to the contract
2. **Players pay to play**: Each guess costs a small fee (added to the pot)
3. **Winner takes all**: Correct guesses win the entire accumulated pot
4. **New round starts**: Admin can reset with fresh funding

This creates real stakes and makes the game much more engaging!

## Step 1: 🪙 Add Token Handling

First, we need to import Stellar's token functionality. Add the following to your imports at the top of `lib.rs`:

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol};
use stellar_registry::import_asset;
import_asset!(xlm);
```

It contains types and methods for token contracts, like checking balances and making transfers.

## Step 2: 💰 Add Funds to the Contract

Whenever the admin resets the number, we need to transfer some funds to the contract to get the pot started. The easiest way to do this is directly in the `set_random_number` method. Remember, this is the private function we call once in the constructor when we first deploy the contract and again any time the reset method is invoked.

```rust
    fn set_random_number(env: &Env) {
        let new_number: u64 = env.prng().gen_range(1..=10);
        env.storage().instance().set(&THE_NUMBER, &new_number);

        // Seed the initial pot
        let x = xlm::client(env);
        let admin = Self::admin(env).expect("admin not set");
        x.transfer(10_000_000_0, &admin, env.current_contract_address());
    }
```

This creates a client to interact with the XLM contract, gets the admin's address from storage, and then runs a transfer.

_🏗️✨ TODO: explain Registry and calls via client, also stroops_

## Step 3: 🙋 Update the Guess Function

This is the big one! Let's make guessing cost money and pay out winners:

```rust
/// Guess a number between 1 and 10
/// Costs a fee and pays out the entire pot if correct
pub fn guess(env: &Env, guesser: Address, a_number: u64) -> bool {
  // Verify the guesser is actually the one calling this function
  guesser.require_auth();

  if guessed_it = a_number == env.storage().instance().get::<_, u64>(&THE_NUMBER).unwrap() {
    let tx = x.transfer(
      self.current_contract_address(),
      guesser,
      x.balance(self.current_contract_address()),
    );
    if tx.is_err {
      panic!("transfer failed!");
    }
  } else {
    let tx = x.transfer(guesser, self.current_contract_address(), 1_000_000_0);
    if tx.is_err {
      panic!("transfer failed!");
    }
  }

  guessed_it
}
```

## Step 7: Your Complete Updated Contract

Here's your full contract with economic incentives:

_🏗️✨ TODO: add link to github repo_

## Step 8: Test the Economic System

_🏗️✨ TODO: add screenshots of interacting with contract explorer_

Now let's test our new economic features:

### Check the Prize Pot on Deploy

You should see `10000000` (1 XLM)!

### Make Some Paid Guesses

Now let's have different users make guesses:

Use freighter to switch accounts:

_🏗️✨ TODO: add screenshots of interacting with freighter_

Via the CLI:
```bash
# Bob makes a guess (and pays the fee)
stellar contract invoke \
  --id [CONTRACT_ID] \
  --source bob \
  --network local \
  -- guess \
  --guesser $(stellar keys address bob) \
  --a_number 3

# Check the pot after Bob's guess
stellar contract invoke \
  --id [CONTRACT_ID] \
  --source alice \
  --network local \
  -- get_prize_pot
```

The pot should now be `10100000` (1.01 XLM). That's the original 1 XLM plus Bob's 0.01 XLM guess fee.

### Test Winning

Keep guessing with different numbers until someone wins. When someone guesses correctly, they'll receive the entire pot, and the pot will reset to 0.

## 🧪 Update the Tests

_🏗️✨ TODO_

## What We've Learned

### 1. Token Economics in Smart Contracts

- **Fee collection**: Charge users for actions
- **Prize pools**: Accumulate fees for distribution
- **Automatic payouts**: Transfer winnings programmatically

### 2. Cross-Contract Calls

- Token transfers are calls to the native token contract
- `token::Client` provides a convenient interface
- All transfers require proper authentication

### 3. State Management

- Track financial state alongside game state
- Update balances consistently
- Handle edge cases (empty pots, etc.)

## What's Next?

In **Step 4**, we'll add professional polish by:

- Implementing proper error handling with custom error types
- Adding security measures
- Adding events for better monitoring
- 🚀 Deploy to mainnet!

This final step will transform our fun game into production-ready code!
