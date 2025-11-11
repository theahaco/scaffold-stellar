use soroban_sdk::{Env, String};

use super::to_str::AsStr;
use crate::Error;

pub(crate) struct Normalized {
    len: usize,
    internal: [u8; Self::MAX_NAME_LENGTH],
}

impl Normalized {
    pub const MAX_NAME_LENGTH: usize = 64;

    pub fn canonicalize(s: &String) -> Result<String, Error> {
        Normalized::new(s)?.to_string(s.env())
    }

    pub fn new(s: &String) -> Result<Self, Error> {
        let len = s.len() as usize;
        if len > Self::MAX_NAME_LENGTH || s.is_empty() {
            return Err(Error::InvalidName);
        }
        let mut internal = [0u8; Self::MAX_NAME_LENGTH];
        let (first, _) = internal.split_at_mut(len);
        s.copy_into_slice(first);
        Self { len, internal }.normalize()?.validate()
    }

    fn normalize(mut self) -> Result<Self, Error> {
        let s = self.as_mut_str()?;
        if !s.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return Err(Error::InvalidName);
        }
        let mut chars_to_change: [Option<(usize, char)>; Self::MAX_NAME_LENGTH] =
            [None; Self::MAX_NAME_LENGTH];
        for (i, c) in s.chars().enumerate() {
            if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                return Err(Error::InvalidName);
            }
            if c == '_' {
                chars_to_change[i] = Some((i, '-'));
            }
            if c.is_ascii_uppercase() {
                chars_to_change[i] = Some((i, c.to_ascii_lowercase()));
            }
        }
        let as_bytes = unsafe { s.as_bytes_mut() };
        for (i, c) in chars_to_change.into_iter().flatten() {
            as_bytes[i] = c as u8;
        }
        Ok(self)
    }

    fn validate(self) -> Result<Self, Error> {
        if is_keyword(self.as_str()?) {
            return Err(Error::InvalidName);
        }
        Ok(self)
    }

    fn as_bytes(&self) -> &[u8] {
        let (first, _) = self.internal.split_at(self.len);
        first
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        let (first, _) = self.internal.split_at_mut(self.len);
        first
    }

    pub fn to_string(&self, env: &Env) -> Result<String, Error> {
        let s = self.as_str()?;
        Ok(String::from_str(env, s))
    }
}

impl AsStr for Normalized {
    fn as_mut_str(&mut self) -> Result<&mut str, Error> {
        self.as_mut_bytes().as_mut_str()
    }

    fn as_str(&self) -> Result<&str, Error> {
        self.as_bytes().as_str()
    }
}

/// from crate `check_keyword`
/// <https://github.com/JoelCourtney/check_keyword/blob/68486cbfa368070fdbfd383fc5840aa380bb1e6f/src/lib.rs#L120>
fn is_keyword(s: &str) -> bool {
    match s {
    "as" |
    "break" |
    "const" |
    "continue" |
    "crate" |
    "else" |
    "enum" |
    "extern" |
    "false" |
    "fn" |
    "for" |
    "if" |
    "impl" |
    "in" |
    "let" |
    "loop" |
    "match" |
    "mod" |
    "move" |
    "mut" |
    "pub" |
    "ref" |
    "return" |
    "self" |
    "Self" |
    "static" |
    "struct" |
    "super" |
    "trait" |
    "true" |
    "type" |
    "unsafe" |
    "use" |
    "where" |
    "while" |

    // STRICT, 2018

    "async"|
    "await"|

    // DYN

    "dyn" |

    // RESERVED, 2015

    "abstract" |
    "become" |
    "box" |
    "do" |
    "final" |
    "macro" |
    "override" |
    "priv" |
    "typeof" |
    "unsized" |
    "virtual" |
    "yield" |

    // RESERVED, 2018

    "try" |

    // RESERVED, 2024
    "gen" |

    // WEAK

    "macro_rules" |
    "union" |
    "'static" |

    // Windows keywords
    "nul" => true,
    _ => false
    }
}
