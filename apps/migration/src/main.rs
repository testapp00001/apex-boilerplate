//! Migration CLI tool.

use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    cli::run_cli(migration::Migrator).await;
}
