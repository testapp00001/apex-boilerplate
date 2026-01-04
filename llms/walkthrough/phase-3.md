# Apex Rust Boilerplate Walkthrough

## Phase 1: Workspace Foundation ✅

Hexagonal Architecture with 4 crates + migration tool.

---

## Phase 2: Database Layer ✅

- [DatabaseConnections](file:///home/kaiser/projects/demo-project/apex-project/crates/apex-infra/src/database/connections.rs#50-56) (main 100+ pool, secondary <20 pool)
- SeaORM entities and migrations
- PostgresUserRepository with full CRUD

---

## Phase 3: API & Middleware ✅

### Graceful Shutdown
Server handles SIGINT/SIGTERM and drains connections before stopping.

### JWT Authentication

**Register** → `POST /api/auth/register`
```json
{"email":"test@example.com","password":"securepassword123"}
→ {"access_token":"eyJ...","token_type":"Bearer","expires_in":86400}
```

**Protected Routes** → Use [Identity](file:///home/kaiser/projects/demo-project/apex-project/apps/api-server/src/middleware/auth.rs#18-23) extractor:
```rust
async fn protected(identity: Identity) -> impl Responder {
    format!("Hello, {}!", identity.email)
}
```

### RFC 7807 Error Responses
```json
{
  "type": "about:blank",
  "title": "Authentication Required",
  "status": 401,
  "detail": "Please provide a valid Bearer token..."
}
```

### Rate Limiting
- In-memory rate limiter using governor (GCRA algorithm)
- Configurable via `RATE_LIMIT_MAX_REQUESTS` and `RATE_LIMIT_WINDOW_SECS`
- Falls back gracefully if rate limiter errors

### Verification

```bash
# Health check
curl http://127.0.0.1:8080/api/health
# {"status":"ok","version":"0.1.0"...}

# Register
curl -X POST .../api/auth/register -d '{"email":"...","password":"..."}'
# Returns JWT token

# Protected route (with token)
curl .../api/auth/me -H "Authorization: Bearer <token>"
# {"id":"...","email":"test@example.com"...}

# Protected route (without token)
curl .../api/auth/me
# {"status":401,"title":"Authentication Required"...}
```

---

## Next: Phase 4 - Observability

- Request ID middleware
- OpenTelemetry integration
- Critical error alerting layer
