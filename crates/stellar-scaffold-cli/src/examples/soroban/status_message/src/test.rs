#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Events, vec, Env, IntoVal};

#[test]
fn test() {
    let env = Env::default();
    let contract_id = env.register(StatusMessageContract, ());
    let client = StatusMessageContractClient::new(&env, &contract_id);

    // Create a test user address
    let user1 = Address::generate(&env);
    let message1 = String::from_str(&env, "Hello, Soroban!");

    // Test getting a non-existent message
    assert_eq!(client.messages_get(&user1), None);

    // Test setting a message (needs authentication)
    env.mock_auth(vec![&env, &user1], || {
        client.messages_set(&user1, &message1);
    });

    // Verify the message was set
    assert_eq!(client.messages_get(&user1), Some(message1.clone()));

    // Verify the event was emitted
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                (user1.clone(), symbol_short!("message_set")).into_val(&env),
                message1.clone().into_val(&env)
            ),
        ]
    );

    // Test updating an existing message
    let message2 = String::from_str(&env, "Updated message!");
    env.mock_auth(vec![&env, &user1], || {
        client.messages_set(&user1, &message2);
    });

    // Verify the message was updated
    assert_eq!(client.messages_get(&user1), Some(message2.clone()));

    // Verify the new event was emitted
    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id,
                (user1.clone(), symbol_short!("message_set")).into_val(&env),
                message2.into_val(&env)
            ),
        ]
    );

    // Test attempting to set a message for another user (should panic)
    let user2 = Address::generate(&env);
    let message3 = String::from_str(&env, "Unauthorized message!");
    
    // This should panic because user1 is trying to set user2's message
    let result = std::panic::catch_unwind(|| {
        env.mock_auth(vec![&env, &user1], || {
            client.messages_set(&user2, &message3);
        });
    });
    assert!(result.is_err());
}
