use soroban_sdk::Env;

use crate::{
    name::NormalizedName,
    test::{
        registry::to_string,
        util::{invalid_string, valid_string},
    },
};

#[test]
fn valid_simple() {
    valid_string("publish");
    valid_string("a_a_b");
    valid_string("abcdefghabcdefgh");
    valid_string("abcdefghabcdefghabcdefghabcdefgh");
}

#[test]
fn valid_complex() {
    valid_string("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh");
    valid_string("a-a_b");
}

#[test]
fn invalid_keywords() {
    invalid_string("pub");
    invalid_string("Pub");
    invalid_string("PUb");

    invalid_string("enum");
    invalid_string("eNum");
}

#[test]
fn invalid() {
    invalid_string("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgha");
    invalid_string("a-a]]]_b");
    invalid_string("_ab");
    invalid_string("-ab");
    invalid_string("1ab");
}

#[test]
fn normalization() {
    assert_eq!(
        NormalizedName::new(&to_string(&Env::default(), "ls_test"))
            .unwrap()
            .to_string(),
        to_string(&Env::default(), "ls-test")
    );
    assert_eq!(
        NormalizedName::new(&to_string(&Env::default(), "ls-test"))
            .unwrap()
            .to_string(),
        to_string(&Env::default(), "ls-test")
    );

    assert_eq!(
        NormalizedName::new(&to_string(&Env::default(), "Test"))
            .unwrap()
            .to_string(),
        to_string(&Env::default(), "test")
    );
    assert_eq!(
        NormalizedName::new(&to_string(&Env::default(), "Ls-teSt"))
            .unwrap()
            .to_string(),
        to_string(&Env::default(), "ls-test")
    );
}
