use loam_sdk::soroban_sdk::String;

use crate::Error;

const MAX_VERSION_LENGTH: usize = 200;

pub fn parse(s: &String) -> Result<semver::Version, Error> {
    if s.len() as usize > MAX_VERSION_LENGTH || s.is_empty() {
        return Err(Error::InvalidVersion);
    }
    let mut out = [0u8; MAX_VERSION_LENGTH];
    let (first, _) = out.split_at_mut(s.len() as usize);
    s.copy_into_slice(first);
    let Ok(s) = core::str::from_utf8(first) else {
        return Err(Error::InvalidVersion);
    };
    Ok(s.parse().map_err(|_| Error::InvalidVersion)?)
}
