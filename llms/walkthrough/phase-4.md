# Apex Rust Boilerplate Walkthrough

## Phases 1-3 ✅ (Complete)

Foundation, Database, and API & Middleware layers implemented.

---

## Phase 4: Observability ✅

### Structured Logging

Configurable via `LOG_FORMAT` environment variable:
- `pretty` (default) - Human-readable for development
- `json` - Structured JSON for production log aggregation

### Request ID Middleware

Every request gets a unique ID:
```bash
curl -I localhost:8080/api/health
# x-request-id: 63d16d54-88a3-4a25-867b-eea8cb0e88f0

# Client-provided IDs are preserved:
curl -I localhost:8080/api/health -H "X-Request-ID: my-custom-id"
# x-request-id: my-custom-id
```

Access in handlers:
```rust
use crate::observability::RequestId;

async fn handler(request_id: RequestId) -> impl Responder {
    tracing::info!(request_id = %request_id.as_str(), "Processing");
    // ...
}
```

### OpenTelemetry (Optional)

Enable with `--features otel`:
```bash
OTEL_ENABLED=true \
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
cargo run -p api-server --features otel
```

### Critical Error Alerting

Automatic alerts on ERROR-level events:

```rust
// This triggers an alert:
tracing::error!("Database connection failed");
```

Configure webhook (Slack, Discord, etc.):
```bash
ALERTS_ENABLED=true
ALERT_WEBHOOK_URL=https://hooks.slack.com/services/xxx
```

### Verification

```bash
# Server starts with telemetry info:
# INFO api_server::telemetry: Telemetry initialized
#   service: apex-api
#   json_logs: false
#   alerts_enabled: true

# Request IDs in headers:
curl -I localhost:8080/api/health
# x-request-id: <uuid>
```

---

## Next: Phase 5 - Background Jobs & Real-time

- Job queue abstraction (Redis Streams / in-memory)
- Cron scheduling with distributed lock
- WebSockets with Socketioxide
