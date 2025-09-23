use loam_sdk::soroban_sdk::String;

use crate::error::Error;

const MAX_NAME_LENGTH: usize = 64;

pub(crate) fn canonicalize(s: &String) -> Result<String, Error> {
    let env = s.env();
    if s.len() as usize > MAX_NAME_LENGTH || s.is_empty() {
        return Err(Error::InvalidName);
    }
    let mut out = [0u8; MAX_NAME_LENGTH];
    let (first, _) = out.split_at_mut(s.len() as usize);
    s.copy_into_slice(first);
    let s = core::str::from_utf8_mut(first).map_err(|_| Error::InvalidName)?;
    if is_keyword(s) || !s.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err(Error::InvalidName);
    }
    let mut chars_to_change: [Option<(usize, char)>; MAX_NAME_LENGTH] = [None; MAX_NAME_LENGTH];
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
    Ok(String::from_bytes(env, as_bytes))
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
