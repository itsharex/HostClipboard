use ahash::AHasher;
use std::hash::{Hash, Hasher};

pub fn hash_str(input: &str) -> u64 {
    let mut hasher = AHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

pub fn hash_vec(input: &[u8]) -> u64 {
    let mut hasher = AHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}