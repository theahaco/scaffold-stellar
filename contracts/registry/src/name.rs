use loam_sdk::soroban_sdk::String;

use crate::error::Error;

pub(crate) fn is_valid(s: &String) -> Option<String> {
    let env = s.env();
    if s.len() > 64 || s.is_empty() {
        return None;
    }
    let mut out = [0u8; 64];
    let (first, _) = out.split_at_mut(s.len() as usize);
    s.copy_into_slice(first);
    let Ok(s) = core::str::from_utf8_mut(first) else {
        return None;
    };
    if is_keyword(s) || !s.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut chars_to_change: [Option<usize>; 64] = [None; 64];
    for (i, c) in s.chars().enumerate() {
        if !(c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return None;
        }
        if c == '_' {
            chars_to_change[i] = Some(i);
        }
    }
    let as_bytes = unsafe { s.as_bytes_mut() };
    for i in chars_to_change.into_iter().filter_map(|x| x) {
        as_bytes[i] = b'-';
    }
    Some(String::from_bytes(env, as_bytes))
}

pub(crate) fn canonicalize(s: &String) -> Result<String, Error> {
    is_valid(s).ok_or(Error::InvalidName)
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
