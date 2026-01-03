# Apex Rust Boilerplate - Implementation Plan

A comprehensive, production-ready Rust backend boilerplate using **Actix-web**, **SeaORM**, and **Hexagonal Architecture**. Designed for flexibility, modularity, and graceful degradation.

## User Review Required

> [!IMPORTANT]
> **Database Choice**: This plan assumes PostgreSQL as the primary database. Confirm if you need MySQL/SQLite support from day one.

> [!IMPORTANT]
> **Redis**: Redis is optional (feature-flagged). Without it, the system falls back to in-memory caching, rate limiting, and job queues. Want Redis from the start or add it later?

---

## Proposed Changes

### Phase 1: Workspace Foundation

#### [MODIFY] [Cargo.toml](file:///home/kaiser/projects/demo-project/apex-project/Cargo.toml)
Convert to workspace root. Define shared dependencies and feature flags:
```toml
[workspace]
members = ["crates/*", "apps/*"]
resolver = "2"

[workspace.dependencies]
# Core
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
thiserror = "2"
anyhow = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Web
actix-web = "4"
actix-rt = "2"

# Database
sea-orm = { version = "1", features = ["runtime-tokio-rustls", "sqlx-postgres"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

[workspace.metadata.features]
default = ["full"]
full = ["postgres", "redis"]
postgres = []
redis = []
minimal = []
```

---

#### [NEW] [crates/apex-core/Cargo.toml](file:///home/kaiser/projects/demo-project/apex-project/crates/apex-core/Cargo.toml)
Domain layer crate - **zero infrastructure dependencies**.

#### [NEW] [crates/apex-core/src/lib.rs](file:///home/kaiser/projects/demo-project/apex-project/crates/apex-core/src/lib.rs)
```rust
pub mod domain;   // Entities (User, etc.)
pub mod ports;    // Traits (UserRepository, Cache, etc.)
pub mod error;    // Domain errors
```

#### [NEW] [crates/apex-core/src/ports/repository.rs](file:///home/kaiser/projects/demo-project/apex-project/crates/apex-core/src/ports/repository.rs)
Repository trait definition:
```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, RepoError>;
    async fn save(&self, user: User) -> Result<User, RepoError>;
}
```

---

#### [NEW] [crates/apex-infra/Cargo.toml](file:///home/kaiser/projects/demo-project/apex-project/crates/apex-infra/Cargo.toml)
Infrastructure implementations with feature flags.

#### [NEW] [crates/apex-infra/src/database/connections.rs](file:///home/kaiser/projects/demo-project/apex-project/crates/apex-infra/src/database/connections.rs)
Multi-database connection manager (as discussed):
```rust
pub struct DatabaseConnections {
    pub main: DbConn,              // 100+ pool
    pub secondary: Vec<NamedConnection>, // <20 pool each
}
```

---

#### [NEW] [crates/apex-shared/Cargo.toml](file:///home/kaiser/projects/demo-project/apex-project/crates/apex-shared/Cargo.toml)
Shared DTOs and validation logic.

---

#### [NEW] [apps/api-server/Cargo.toml](file:///home/kaiser/projects/demo-project/apex-project/apps/api-server/Cargo.toml)
Actix-web binary crate.

#### [NEW] [apps/api-server/src/main.rs](file:///home/kaiser/projects/demo-project/apex-project/apps/api-server/src/main.rs)
Application entry point with dependency injection wiring.

#### [DELETE] [src/main.rs](file:///home/kaiser/projects/demo-project/apex-project/src/main.rs)
Will be replaced by `apps/api-server/src/main.rs`.

---

### Final Directory Structure

```
apex-project/
├── Cargo.toml              # Workspace root
├── Cargo.lock
├── .env.example
├── crates/
│   ├── apex-core/          # Domain layer (pure Rust)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── domain/     # Entities
│   │       ├── ports/      # Traits
│   │       └── error.rs
│   ├── apex-infra/         # Infrastructure layer
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── database/   # SeaORM implementations
│   │       └── cache/      # Redis/In-memory
│   └── apex-shared/        # DTOs, validation
│       └── src/
│           └── lib.rs
├── apps/
│   └── api-server/         # Actix-web entry point
│       └── src/
│           ├── main.rs
│           ├── handlers/
│           └── middleware/
└── llms/                   # (existing) Research docs
```

---

## Verification Plan

### Automated Tests

**1. Workspace Compilation**
```bash
cd /home/kaiser/projects/demo-project/apex-project
cargo build --workspace
```
Expected: All crates compile without errors.

**2. Feature Flag Testing**
```bash
# Full mode (default)
cargo build --workspace

# Minimal mode (no Redis/Postgres)
cargo build --workspace --no-default-features --features minimal
```
Expected: Both configurations compile successfully.

**3. Run API Server**
```bash
cargo run -p api-server
```
Expected: Server starts, health endpoint returns `200 OK`.

### Manual Verification

1. **Health Check**: After server starts, visit `http://localhost:8080/health` — should return JSON `{"status": "ok"}`.
2. **Structure Review**: Verify directory structure matches the proposed layout.

---

## Questions Before Proceeding

1. **Port**: Do you want the API server on `8080` (default) or a different port?
2. **Environment**: Should I include `.env.example` with database URL templates?
3. **Phase Scope**: Should I implement all 6 phases, or start with Phase 1 only and iterate?
