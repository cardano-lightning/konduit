use rand::RngCore;
use rand::rngs::OsRng;

/// TODO :: Our version of random is ancient.
pub fn arr32() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    let mut rng = OsRng::new().expect("OS RNG unavailable");
    rng.try_fill_bytes(&mut bytes).expect("OS RNG failed");
    bytes
}
