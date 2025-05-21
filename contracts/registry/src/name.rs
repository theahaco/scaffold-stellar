use loam_sdk::soroban_sdk::String;

use crate::error::Error;

pub(crate) fn is_valid(s: &String) -> bool {
    if s.len() > 64 || s.is_empty() {
        return false;
    }
    let mut out = [0u8; 64];
    let (first, _) = out.split_at_mut(s.len() as usize);
    s.copy_into_slice(first);
    let Ok(s) = core::str::from_utf8(first) else {
        return false;
    };
    if is_keyword(s) || s.starts_with(|c: char| c == '_' || c.is_numeric()) {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(crate) fn validate(s: &String) -> Result<(), Error> {
    is_valid(s).then_some(()).ok_or(Error::InvalidName)
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
