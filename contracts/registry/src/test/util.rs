use soroban_sdk::Env;

use crate::{name::canonicalize, test::registry::to_string, Error};
pub fn valid_string(s: &str) {
    test_string(s, true);
}

pub fn invalid_string(s: &str) {
    test_string(s, false);
}

pub fn test_string(s: &str, result: bool) {
    let raw_result = canonicalize(&to_string(&Env::default(), s));
    if result {
        assert!(raw_result.is_ok(), "should be valid: {s}");
    } else {
        assert_eq!(
            raw_result,
            Err(Error::InvalidName),
            "should be invalid: {s}"
        );
    }
}
