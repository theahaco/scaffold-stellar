#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub count: u32,
    pub last_incr: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum Asset {
    Stellar(soroban_sdk::Address),
    Other(soroban_sdk::Symbol),
}

const STATE: Symbol = symbol_short!("STATE");
const TEST_CONFIG: Symbol = symbol_short!("TESTCFG");

#[contracttype]
pub struct TestConfig {
    pub assets: Vec<Asset>,
    pub base: Asset,
    pub decimals: u32,
    pub resolution: u32,
}

#[contract]
pub struct IncrementContract;

#[contractimpl]
impl IncrementContract {
    /// test init
    pub fn test_init(
        env: Env,
        assets: Vec<Asset>,
        base: Asset,
        decimals: u32,
        resolution: u32,
    ) {
        if env.storage().instance().has(&TEST_CONFIG) {
            panic!("test already initialized");
        }

        let config = TestConfig {
            assets,
            base,
            decimals,
            resolution,
        };

        env.storage().instance().set(&TEST_CONFIG, &config);
    }

    /// get test config
    pub fn get_test_config(env: Env) -> TestConfig {
        env.storage().instance().get(&TEST_CONFIG).unwrap()
    }
    /// Increment increments an internal counter, and returns the value.
    pub fn increment(env: Env, incr: u32) -> u32 {
        // Get the current count.
        let mut state = Self::get_state(env.clone());

        // Increment the count.
        state.count += incr;
        state.last_incr = incr;

        // Save the count.
        env.storage().instance().set(&STATE, &state);

        // Return the count to the caller.
        state.count
    }
    /// Return the current state.
    pub fn get_state(env: Env) -> State {
        env.storage().instance().get(&STATE).unwrap_or(State {
            count: 0,
            last_incr: 0,
        }) // If no value set, assume 0.
    }
}

mod test;
