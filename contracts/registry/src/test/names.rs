extern crate std;

use std::{format, vec::Vec};

use soroban_sdk::Env;

use crate::{error::Error, name::NormalizedName, test::registry::to_string};

#[rustfmt::skip]
const VALID_NAMES: &[&str] = &[
    // Simple cases
    "publish",
    "a_a_b",
    "abcdefghabcdefgh",                 // 16 chars
    "abcdefghabcdefghabcdefghabcdefgh", // 32 chars
    // Complex cases
    "abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh", // 64 chars (max)
    "a-a_b",
    "hello-world",
    "myContract123",
    "A",
    "z",
];

#[test]
fn valid_names() {
    let env = Env::default();
    let mut failures = Vec::new();

    for name in VALID_NAMES {
        let result = NormalizedName::new(&to_string(&env, name));
        if result.is_err() {
            failures.push(format!("should be valid: {name}"));
        }
    }

    assert!(failures.is_empty(), "failures:\n  {}", failures.join("\n  "));
}

const INVALID_NAMES: &[&str] = &[
    // Too long (65 chars)
    "abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgha",
    // Invalid characters
    "a-a]]]_b",
    "hello world",
    "hello@world",
    "hello.world",
    // Must start with alphabetic
    "_ab",
    "-ab",
    "1ab",
    "123",
    // Empty
    "",
];

#[test]
fn invalid_names() {
    let env = Env::default();
    let mut failures = Vec::new();

    for name in INVALID_NAMES {
        let result = NormalizedName::new(&to_string(&env, name));
        if result != Err(Error::InvalidName) {
            failures.push(format!("should be invalid: '{name}'"));
        }
    }

    assert!(failures.is_empty(), "failures:\n  {}", failures.join("\n  "));
}

#[rustfmt::skip]
const RUST_KEYWORDS: &[&str] = &[
    // Case variations
    "pub",
    "Pub",
    "PUB",
    "enum",
    "eNum",
    // Standard keywords
    "struct",
    "fn",
    "let",
    "mut",
    "const",
    "async",
    "await",
    "self",
    "Self",
    "impl",
    "trait",
    "type",
    "where",
    "for",
    "loop",
    "while",
    "if",
    "else",
    "match",
    "return",
    "break",
    "continue",
    "move",
    "ref",
    "static",
    "super",
    "crate",
    "mod",
    "use",
    "as",
    "in",
    "extern",
    "dyn",
    "unsafe",
    "true",
    "false",
    // Reserved keywords
    "try",
    "gen",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "union",
    // Windows reserved
    "nul",
];

#[test]
fn invalid_keywords() {
    let env = Env::default();
    let mut failures = Vec::new();

    for name in RUST_KEYWORDS {
        let result = NormalizedName::new(&to_string(&env, name));
        if result != Err(Error::InvalidName) {
            failures.push(format!("keyword should be invalid: '{name}'"));
        }
    }

    assert!(failures.is_empty(), "failures:\n  {}", failures.join("\n  "));
}

// (input, expected)
const NORMALIZATION_CASES: &[(&str, &str)] = &[
    ("ls_test", "ls-test"),
    ("ls-test", "ls-test"),
    ("Test", "test"),
    ("Ls-teSt", "ls-test"),
    ("HELLO_WORLD", "hello-world"),
    ("My_Cool_Contract", "my-cool-contract"),
    ("ABC", "abc"),
];

#[test]
fn normalization() {
    let env = Env::default();
    let mut failures = Vec::new();

    for (input, expected) in NORMALIZATION_CASES {
        match NormalizedName::new(&to_string(&env, input)) {
            Ok(normalized) => {
                let actual = normalized.to_string();
                let expected_str = to_string(&env, expected);
                if actual != expected_str {
                    failures.push(format!("'{input}' should normalize to '{expected}', got '{actual}'"));
                }
            }
            Err(_) => {
                failures.push(format!("'{input}' should be valid but failed"));
            }
        }
    }

    assert!(failures.is_empty(), "failures:\n  {}", failures.join("\n  "));
}
