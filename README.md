# Apex - Production-Ready Rust Backend Boilerplate

A flexible, modular Rust backend boilerplate following **Hexagonal Architecture** with support for multiple databases, real-time features, and enterprise-grade observability.

## ✨ Features

| Feature                       | Description                                                        |
| ----------------------------- | ------------------------------------------------------------------ |
| 🏗️ **Hexagonal Architecture** | Clean separation of domain, infrastructure, and application layers |
| 🗄️ **Multi-Database Support** | Main + secondary database pattern with connection pooling          |
| 🔐 **JWT Authentication**     | Argon2 password hashing + JWT tokens                               |
| ⚡ **Rate Limiting**          | In-memory rate limiter with GCRA algorithm                         |
| 📡 **Real-time WebSockets**   | Socketioxide with room support                                     |
| 🔄 **Background Jobs**        | In-memory job queue with workers and retries                       |
| ⏰ **Cron Scheduling**        | tokio-cron-scheduler integration                                   |
| 📊 **Observability**          | Structured logging, request IDs, OpenTelemetry                     |
| 🚨 **Alerting**               | Critical error notifications (console/webhook)                     |

## 🚀 Quick Start

```bash
# Clone and enter project
cd apex-project

# Copy environment file
cp .env.example .env

# Run in development mode
cargo run -p api-server

# With PostgreSQL
DATABASE_URL=postgres://user:pass@localhost:5432/apex cargo run -p api-server

# Run migrations
cargo run -p migration -- up
```

## 📦 Project Structure

```
apex-project/
├── crates/
│   ├── apex-core/      # Domain layer (entities, traits, errors)
│   ├── apex-infra/     # Infrastructure (DB, cache, services)
│   └── apex-shared/    # Shared DTOs and response types
├── apps/
│   ├── api-server/     # HTTP server application
│   └── migration/      # Database migrations
└── Cargo.toml          # Workspace configuration
```

## 🎛️ Feature Flags

### API Server

```bash
# Full features (default)
cargo run -p api-server

# Minimal (no external deps)
cargo run -p api-server --no-default-features --features minimal

# Custom selection
cargo run -p api-server --no-default-features --features "postgres,auth"

# With OpenTelemetry
cargo run -p api-server --features otel
```

| Feature      | Description                    |
| ------------ | ------------------------------ |
| `full`       | All features enabled (default) |
| `minimal`    | Bare HTTP server only          |
| `postgres`   | PostgreSQL via SeaORM          |
| `auth`       | JWT + Argon2 authentication    |
| `rate-limit` | Request rate limiting          |
| `scheduler`  | Cron job scheduling            |
| `websocket`  | WebSocket support              |
| `otel`       | OpenTelemetry tracing          |

## 🔧 Configuration

All configuration via environment variables:

```bash
# Server
HOST=127.0.0.1
PORT=8080

# Database
DATABASE_URL=postgres://user:password@localhost:5432/apex_db
DB_MAX_CONNECTIONS=100

# Authentication
JWT_SECRET=your-secret-key
JWT_EXPIRATION_HOURS=24

# Rate Limiting
RATE_LIMIT_MAX_REQUESTS=100
RATE_LIMIT_WINDOW_SECS=60

# Logging
RUST_LOG=info,api_server=debug
LOG_FORMAT=pretty  # or "json"

# Alerting
ALERTS_ENABLED=true
ALERT_WEBHOOK_URL=https://hooks.slack.com/...
```

## 📡 API Endpoints

```bash
# Health check
GET /api/health

# Authentication
POST /api/auth/register  # {"email": "...", "password": "..."}
POST /api/auth/login     # {"email": "...", "password": "..."}
GET  /api/auth/me        # Requires: Authorization: Bearer <token>
```

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                   API Server                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │  Handlers   │  │ Middleware  │  │   Routes    │  │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │
└─────────┼────────────────┼────────────────┼─────────┘
          │                │                │
┌─────────┼────────────────┴────────────────┼─────────┐
│         ▼        Application Layer        ▼         │
│  ┌─────────────────────────────────────────────┐    │
│  │              Use Cases / Services            │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────┬───────────────────────────┘
                          │
┌─────────────────────────┴───────────────────────────┐
│                    Domain Layer                      │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐     │
│  │  Entities  │  │   Traits   │  │   Errors   │     │
│  │   (User)   │  │(Repository)│  │ (Domain)   │     │
│  └────────────┘  └────────────┘  └────────────┘     │
└─────────────────────────┬───────────────────────────┘
                          │
┌─────────────────────────┴───────────────────────────┐
│               Infrastructure Layer                   │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌────────┐  │
│  │PostgreSQL│  │  Cache  │  │  Auth   │  │ Jobs   │  │
│  │  Repo    │  │(Memory) │  │ (JWT)   │  │(Queue) │  │
│  └─────────┘  └─────────┘  └─────────┘  └────────┘  │
└─────────────────────────────────────────────────────┘
```

## 📝 License

MIT License - see [LICENSE](LICENSE) for details.
