use soroban_sdk::{Env, String};

use crate::error::Error;

mod normalized;
mod to_str;

use normalized::Normalized;

pub(crate) const REGISTRY: &str = "registry";

#[must_use]
pub fn registry(env: &Env) -> String {
    String::from_str(env, REGISTRY)
}

/// Checks that the name is a valid crate name.
/// 1. The name must be non-empty.
/// 2. The first character must be an ASCII character.
/// 3. The remaining characters must be ASCII alphanumerics or `-` or `_`.
///
/// Also converts all `_` characters to `-` and makes all alphabet characters lower case to to have a canonical form.
///
/// Then checks if the canonical form is not a rust keyword.
///
/// See: <https://github.com/rust-lang/crates.io/blob/ad7740c951d9876a7070435a47ae11f1b1dc37e4/crates/crates_io_database/src/models/krate.rs#L218>
pub(crate) fn canonicalize(s: &String) -> Result<String, Error> {
    Normalized::canonicalize(s)
}
