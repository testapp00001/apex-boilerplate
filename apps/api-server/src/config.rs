//! Application configuration loaded from environment variables.

use std::env;

use apex_infra::database::{DatabaseConfig, SecondaryDbConfig};

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database: Option<DatabaseConfig>,
}

impl AppConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let database = env::var("DATABASE_URL").ok().map(|main_url| {
            // Parse secondary databases from SECONDARY_DB_* env vars
            let secondary_databases = Self::parse_secondary_databases();

            DatabaseConfig {
                main_url,
                main_max_connections: env::var("DB_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(100),
                main_min_connections: env::var("DB_MIN_CONNECTIONS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
                secondary_databases,
            }
        });

        Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database,
        }
    }

    /// Parse secondary databases from environment.
    /// Format: SECONDARY_DB_<NAME>=<URL>,<MAX_CONNECTIONS>
    /// Example: SECONDARY_DB_ANALYTICS=postgres://...,20
    fn parse_secondary_databases() -> Vec<SecondaryDbConfig> {
        let mut secondary = Vec::new();

        for (key, value) in env::vars() {
            if let Some(name) = key.strip_prefix("SECONDARY_DB_") {
                let parts: Vec<&str> = value.splitn(2, ',').collect();
                if let Some(url) = parts.first() {
                    let max_connections = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);

                    secondary.push(SecondaryDbConfig {
                        name: name.to_lowercase(),
                        url: url.to_string(),
                        max_connections,
                    });
                }
            }
        }

        secondary
    }
}
