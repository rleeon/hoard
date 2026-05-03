use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};
use rand::Rng;
use sha2::{Digest, Sha256};

pub fn hash_password(plain: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    // Memory: 19 MiB, iterations: 2, parallelism: 1
    let params = Params::new(19 * 1024, 2, 1, None)
        .map_err(|e| anyhow::anyhow!("argon2 params error: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let hash = argon2
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hashing failed: {e}"))?;
    Ok(hash.to_string())
}

pub fn verify_password(plain: &str, hash: &str) -> Result<bool> {
    let parsed =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok())
}

pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    format!("hoard_v1_{}", hex::encode(bytes))
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn password_roundtrip() {
        let hash = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn token_uniqueness() {
        let tokens: HashSet<String> = (0..1000).map(|_| generate_token()).collect();
        assert_eq!(tokens.len(), 1000);
    }

    #[test]
    fn token_format() {
        let t = generate_token();
        assert!(t.starts_with("hoard_v1_"));
        assert_eq!(t.len(), 9 + 64); // prefix + 64 hex chars
    }
}
