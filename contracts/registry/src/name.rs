use soroban_sdk::{Env, IntoVal, String, TryFromVal, Val};

use crate::error::Error;

mod normalized;
mod to_str;

use normalized::Normalized;

pub(crate) const REGISTRY: &str = "registry";

#[derive(Clone, PartialEq, Eq)]
pub struct NormalizedName(String);

impl AsRef<String> for NormalizedName {
    fn as_ref(&self) -> &String {
        self.as_string()
    }
}

impl NormalizedName {
    pub unsafe fn new(s: String) -> Self {
        NormalizedName(s)
    }

    pub fn as_string(&self) -> &String {
        &self.0
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl TryFrom<&String> for NormalizedName {
    type Error = Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Ok(Self(canonicalize(value)?))
    }
}

impl TryFrom<String> for NormalizedName {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl IntoVal<Env, Val> for NormalizedName {
    fn into_val(&self, env: &Env) -> Val {
        self.0.into_val(env)
    }
}

impl TryFromVal<Env, Val> for NormalizedName {
    type Error = soroban_sdk::Error;

    fn try_from_val(env: &Env, v: &Val) -> Result<Self, soroban_sdk::Error> {
        let name: String = TryFromVal::try_from_val(env, v)?;
        Ok(Self(name))
    }
}

#[must_use]
pub fn registry(env: &Env) -> NormalizedName {
    unsafe { NormalizedName::new(String::from_str(env, REGISTRY)) }
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
