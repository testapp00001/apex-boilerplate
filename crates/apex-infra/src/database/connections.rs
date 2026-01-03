use std::time::Duration;

#[cfg(feature = "postgres")]
use sea_orm::{ConnectOptions, Database, DbConn, DbErr};

/// Configuration for the main database.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub main_url: String,
    pub main_max_connections: u32,
    pub main_min_connections: u32,
    pub secondary_databases: Vec<SecondaryDbConfig>,
}

/// Configuration for a secondary database.
#[derive(Debug, Clone)]
pub struct SecondaryDbConfig {
    pub name: String,
    pub url: String,
    pub max_connections: u32,
}

/// A named connection to a secondary database.
#[cfg(feature = "postgres")]
pub struct NamedConnection {
    pub name: String,
    pub conn: DbConn,
}

#[cfg(not(feature = "postgres"))]
pub struct NamedConnection {
    pub name: String,
}

/// Multi-database connection manager.
///
/// Provides direct access to the main database and named access to secondary databases.
///
/// # Example
/// ```ignore
/// // Main DB - always available, high connection pool
/// let user = db.main.find_user(id).await?;
///
/// // Secondary DB - named lookup
/// if let Some(analytics) = db.get("analytics") {
///     let report = analytics.query(...).await?;
/// }
/// ```
#[cfg(feature = "postgres")]
pub struct DatabaseConnections {
    /// Primary database - used for most operations (100+ pool).
    pub main: DbConn,
    /// Secondary databases - for specific use cases (<20 pool each).
    pub secondary: Vec<NamedConnection>,
}

#[cfg(not(feature = "postgres"))]
pub struct DatabaseConnections {
    pub secondary: Vec<NamedConnection>,
}

#[cfg(feature = "postgres")]
impl DatabaseConnections {
    /// Initialize all database connections from configuration.
    pub async fn init(config: &DatabaseConfig) -> Result<Self, DbErr> {
        tracing::info!("Initializing database connections...");

        // Main DB: High connection pool for primary operations
        let main_opts = ConnectOptions::new(&config.main_url)
            .max_connections(config.main_max_connections)
            .min_connections(config.main_min_connections)
            .connect_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .sqlx_logging(true)
            .to_owned();

        let main = Database::connect(main_opts).await?;
        tracing::info!(
            "Main database connected (pool: {})",
            config.main_max_connections
        );

        // Secondary DBs: Smaller pools for specific use cases
        let mut secondary = Vec::new();
        for db_config in &config.secondary_databases {
            let opts = ConnectOptions::new(&db_config.url)
                .max_connections(db_config.max_connections)
                .min_connections(2)
                .connect_timeout(Duration::from_secs(10))
                .idle_timeout(Duration::from_secs(300))
                .to_owned();

            let conn = Database::connect(opts).await?;
            tracing::info!(
                "Secondary database '{}' connected (pool: {})",
                db_config.name,
                db_config.max_connections
            );

            secondary.push(NamedConnection {
                name: db_config.name.clone(),
                conn,
            });
        }

        Ok(Self { main, secondary })
    }

    /// Get a secondary database connection by name.
    pub fn get(&self, name: &str) -> Option<&DbConn> {
        self.secondary
            .iter()
            .find(|c| c.name == name)
            .map(|c| &c.conn)
    }

    /// List all available secondary database names.
    pub fn secondary_names(&self) -> Vec<&str> {
        self.secondary.iter().map(|c| c.name.as_str()).collect()
    }
}
