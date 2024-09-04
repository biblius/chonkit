use sha2::{Digest, Sha256};

pub mod parser;
pub mod store;

/// Return a SHA256 hash of the input.
///
/// * `input`: Input bytes.
pub fn sha256(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    Digest::update(&mut hasher, input);
    let out = hasher.finalize();
    hex::encode(out)
}
