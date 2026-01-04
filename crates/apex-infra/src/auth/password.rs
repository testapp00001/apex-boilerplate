//! Argon2 password hashing implementation.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use apex_core::ports::{AuthError, PasswordService};

/// Argon2-based password service.
pub struct Argon2PasswordService {
    argon2: Argon2<'static>,
}

impl Argon2PasswordService {
    pub fn new() -> Self {
        Self {
            argon2: Argon2::default(),
        }
    }
}

impl Default for Argon2PasswordService {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordService for Argon2PasswordService {
    fn hash(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);

        self.argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AuthError::HashingError(e.to_string()))
    }

    fn verify(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::HashingError(e.to_string()))?;

        Ok(self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let service = Argon2PasswordService::new();
        let password = "secure_password_123";

        let hash = service.hash(password).unwrap();
        assert!(service.verify(password, &hash).unwrap());
        assert!(!service.verify("wrong_password", &hash).unwrap());
    }
}
