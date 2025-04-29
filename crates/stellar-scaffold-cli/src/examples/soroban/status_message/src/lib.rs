#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String, symbol_short};

#[contract]
pub struct StatusMessageContract;

#[contractimpl]
impl StatusMessageContract {
    /// Get the message for a specific author
    pub fn messages_get(env: Env, author: Address) -> Option<String> {
        env.storage().instance().get(&author)
    }

    /// Set a message for the author. Only the author can set their own message.
    pub fn messages_set(env: Env, author: Address, text: String) {
        // Require authorization from the author
        author.require_auth();
        
        // Store the message
        env.storage().instance().set(&author, &text);

        // Optionally publish an event about the message being set
        env.events().publish(
            (author, symbol_short!("msg_set")),
            text
        );
    }
}

#[cfg(test)]
mod test;