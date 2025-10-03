use loam_sdk::soroban_sdk::{Bytes, String, crypto::Hash, env};

pub fn hash_string(s: &String) -> Hash<32> {
    let env = env();
    let len = s.len() as usize;
    let mut bytes = [0u8; 100];
    let bytes = &mut bytes[0..len];
    s.copy_into_slice(bytes);
    let mut b = Bytes::new(env);
    b.copy_from_slice(0, bytes);
    env.crypto().sha256(&b)
}

pub const MAX_BUMP: u32 = 535_679;

pub(crate) const REGISTRY: &str = "registry";
