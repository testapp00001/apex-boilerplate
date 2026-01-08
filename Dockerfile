# =============================================================================
# Stage 1: Chef - Cache dependencies
# =============================================================================
FROM lukemathwalker/cargo-chef:latest-rust-slim-trixie AS chef
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    lld \
    && rm -rf /var/lib/apt/lists/*

# =============================================================================
# Stage 2: Planner - Generate recipe.json for dependency caching
# =============================================================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3: Builder - Build release binary
# =============================================================================
FROM chef AS builder

# Copy dependency recipe and build dependencies first (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

# Copy source code and build the actual application
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p api-server && \
    cargo build --release -p migration && \
    cp target/release/api-server . && \
    cp target/release/migration .

# =============================================================================
# Stage 4: Dev - Faster development builds
# =============================================================================
FROM chef AS dev

COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build -p api-server && \
    cargo build -p migration && \
    cp target/debug/api-server . && \
    cp target/debug/migration .

EXPOSE 8080
ENV RUST_LOG=info
CMD ["./api-server"]

# =============================================================================
# Stage 5: Runtime - Minimal production image
# =============================================================================
FROM debian:trixie-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --gid 1000 apex \
    && useradd --uid 1000 --gid apex --shell /bin/bash --create-home apex

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/api-server /app/api-server
COPY --from=builder /app/migration /app/migration

# Copy environment example as reference
COPY .env.example /app/.env.example

# Set ownership
RUN chown -R apex:apex /app

# Switch to non-root user
USER apex

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Default command
CMD ["./api-server"]
