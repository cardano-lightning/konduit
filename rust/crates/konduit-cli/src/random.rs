use rand::{TryRngCore, rngs::OsRng};

pub fn arr32() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.try_fill_bytes(&mut key).unwrap();
    key
}
