# ============================================================
# rsut-pdf-mcp — Multi-stage Docker Build
#
# Stage 1: Build Vue3 SPA frontend
# Stage 2: Build Rust binaries (pdf-mcp + pdf-web)
# Stage 3: Runtime image (minimal debian)
# ============================================================

# ── Stage 1: Frontend (Vue3 SPA) ──
FROM node:20-bookworm AS frontend-builder

WORKDIR /app

# Copy package files for dependency caching
COPY pdf-module-rs/crates/pdf-mcp/pdf-web-ui/package.json \
     pdf-module-rs/crates/pdf-mcp/pdf-web-ui/package-lock.json \
     ./
RUN npm ci

# Copy all frontend source and build
COPY pdf-module-rs/crates/pdf-mcp/pdf-web-ui/ .
RUN npm run build

# ── Stage 2: Backend (Rust) ──
FROM rust:bookworm AS backend-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Vue dist into the correct location for rust-embed
# (must exist before cargo build so rust-embed can include it)
COPY --from=frontend-builder /app/dist crates/pdf-mcp/pdf-web-ui/dist/

# Copy workspace manifests
COPY pdf-module-rs/Cargo.toml pdf-module-rs/Cargo.lock ./
COPY pdf-module-rs/.cargo ./.cargo

# Copy VERSION file for compile-time version injection (build.rs reads it)
COPY VERSION ./

# Copy all crate source
COPY pdf-module-rs/templates ./templates/
COPY pdf-module-rs/crates ./crates

# Build pdf-mcp with embedded Vue SPA
RUN cargo build --release --bin pdf-mcp

# ── Stage 3: Runtime ──
FROM debian:bookworm-slim AS runtime

# Runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1000 pdfuser

WORKDIR /app

# Copy pdf-mcp binary
COPY --from=backend-builder /app/target/release/pdf-mcp /usr/local/bin/pdf-mcp

RUN chmod +x /usr/local/bin/pdf-mcp && \
    mkdir -p /app/data /app/knowledge /app/logs /app/cache && \
    chown -R pdfuser:pdfuser /app

USER pdfuser

# Environment defaults
ENV RUST_LOG=info
ENV HTTP_PORT=8001
ENV KNOWLEDGE_BASE=/app/knowledge
ENV STORAGE_TYPE=local
ENV STORAGE_LOCAL_DIR=/app/data
ENV CACHE_ENABLED=true
ENV CACHE_MAX_SIZE=1000
ENV CACHE_TTL_SECONDS=3600

EXPOSE 8000 8001

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -sf http://localhost:8001/api/health > /dev/null || exit 1

CMD ["pdf-mcp"]
