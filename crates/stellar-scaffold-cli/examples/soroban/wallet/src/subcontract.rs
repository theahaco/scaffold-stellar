use loam_sdk::{
    soroban_sdk::{env, token, Address, Lazy},
    stellar_asset, subcontract,
};

#[derive(Lazy, Default)]
pub struct Calculator;

#[subcontract]
pub trait IsToken {
    /// Get contract's xlm balance
    #[allow(clippy::missing_errors_doc)]
    fn balance(&self) -> i128;

    /// Puts two into into a vector
    fn transfer(&self, to: loam_sdk::soroban_sdk::Address, amount: i128);

    fn addr(&self) -> loam_sdk::soroban_sdk::Address;
}

fn native() -> token::Client<'static> {
    stellar_asset!("native")
}

impl IsToken for Calculator {
    fn balance(&self) -> i128 {
        native().balance(&env().current_contract_address())
    }

    fn transfer(&self, to: Address, amount: i128) {
        native().transfer(&env().current_contract_address(), &to, &amount);
    }
    fn addr(&self) -> loam_sdk::soroban_sdk::Address {
        native().address
    }
}
