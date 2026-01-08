use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User entity - represents a user in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Create a new user with generated ID and timestamps.
    pub fn new(email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            password_hash,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new_creates_valid_user() {
        let email = "test@example.com".to_string();
        let password_hash = "hashed_password".to_string();

        let user = User::new(email.clone(), password_hash.clone());

        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, password_hash);
        assert_ne!(user.id, Uuid::nil());
        assert_eq!(user.created_at, user.updated_at);
    }

    #[test]
    fn test_user_new_generates_unique_ids() {
        let user1 = User::new("user1@test.com".to_string(), "hash1".to_string());
        let user2 = User::new("user2@test.com".to_string(), "hash2".to_string());

        assert_ne!(user1.id, user2.id);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new("test@example.com".to_string(), "hash".to_string());

        let json = serde_json::to_string(&user).expect("Should serialize");
        assert!(json.contains("test@example.com"));

        let deserialized: User = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.id, user.id);
        assert_eq!(deserialized.email, user.email);
    }
}
