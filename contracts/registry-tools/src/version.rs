use loam_sdk::soroban_sdk::{Lazy, String};


const MAX_VERSION_LENGTH: usize = 200;

pub fn parse(s: &String) -> Option<semver::Version> {
    if s.len() as usize > MAX_VERSION_LENGTH || s.is_empty() {
        return None;
    }
    let mut out = [0u8; MAX_VERSION_LENGTH];
    let (first, _) = out.split_at_mut(s.len() as usize);
    s.copy_into_slice(first);
    let Ok(s) = core::str::from_utf8(first) else {
        return None;
    };
    s.parse().ok()
}

fn validate(new: &String, old: Option<&String>) -> Option<()> {
    let version = crate::version::parse(new)?;
    if let Some(current_version) = old {
        if version <= crate::version::parse(current_version)? {
            return None;
        }
    }
    Some(())
}

#[derive(Default)]
pub struct Checker;

impl Lazy for Checker {
    fn get_lazy() -> Option<Self> {
        Some(Checker::default())
    }

    fn set_lazy(self) {}
}

impl IsProperVersion for Checker {
    fn validate_version(&self, new: String, old: Option<String>) -> bool {
        validate(&new, old.as_ref()).is_some()
    }
}

#[loam_sdk::subcontract]
pub trait IsProperVersion {
    /// Fetch the hash of a Wasm binary from the registry
    fn validate_version(
        &self,
        new: loam_sdk::soroban_sdk::String,
        old: Option<loam_sdk::soroban_sdk::String>,
    ) -> bool;
}
