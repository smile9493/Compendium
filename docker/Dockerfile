# ============================================================
# rsut-pdf-mcp — Multi-stage Production Docker Build
#
# Build:
#   docker build -t rsut-pdf-mcp .
#
# Build specific target:
#   docker build --target mcp   -t rsut-pdf-mcp:mcp   .
#   docker build --target web   -t rsut-pdf-mcp:web   .
#   docker build --target cli   -t rsut-pdf-mcp:cli   .
#
# With PDFium:
#   docker build --build-arg WITH_PDFIUM=1 -t rsut-pdf-mcp:full .
#
# Run:
#   docker run -p 8001:8001 -v $(pwd)/kb:/app/kb rsut-pdf-mcp:web
#   docker run -i --rm rsut-pdf-mcp:mcp
# ============================================================

# ─── Stage 0: Planner (extracts dependencies only) ─────────
FROM rust:1.91-bookworm AS planner

WORKDIR /app
COPY pdf-module-rs/Cargo.toml pdf-module-rs/Cargo.lock ./
COPY pdf-module-rs/crates/pdf-common/Cargo.toml ./crates/pdf-common/
COPY pdf-module-rs/crates/pdf-macros/Cargo.toml ./crates/pdf-macros/
COPY pdf-module-rs/crates/pdf-core/Cargo.toml ./crates/pdf-core/
COPY pdf-module-rs/crates/pdf-mcp/Cargo.toml ./crates/pdf-mcp/
COPY pdf-module-rs/crates/pdf-web/Cargo.toml ./crates/pdf-web/
COPY pdf-module-rs/crates/pdf-cli/Cargo.toml ./crates/pdf-cli/
COPY pdf-module-rs/crates/pdf-wasm/Cargo.toml ./crates/pdf-wasm/
COPY pdf-module-rs/crates/vlm-visual-gateway/Cargo.toml ./crates/vlm-visual-gateway/

RUN mkdir -p crates/pdf-common/src crates/pdf-macros/src crates/pdf-core/src \
    crates/pdf-mcp/src crates/pdf-web/src crates/pdf-cli/src \
    crates/pdf-wasm/src crates/vlm-visual-gateway/src && \
    echo 'fn main() {}' > crates/pdf-mcp/src/main.rs && \
    echo 'fn main() {}' > crates/pdf-web/src/main.rs && \
    echo 'fn main() {}' > crates/pdf-cli/src/main.rs && \
    echo 'pub fn dummy() {}' > crates/pdf-common/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/pdf-macros/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/pdf-core/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/pdf-wasm/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/vlm-visual-gateway/src/lib.rs && \
    cargo fetch && \
    cargo check --workspace --all-targets || true

# ─── Stage 1: Backend Builder ──────────────────────────────
FROM rust:1.91-bookworm AS backend-builder

ARG BUILD_PROFILE=release
ARG WITH_PDFIUM=0

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Leverage dependency cache from planner
COPY --from=planner /usr/local/cargo/registry /usr/local/cargo/registry
COPY --from=planner /usr/local/cargo/git /usr/local/cargo/git

# Copy workspace manifests and source
COPY pdf-module-rs/Cargo.toml pdf-module-rs/Cargo.lock ./
COPY pdf-module-rs/.cargo ./.cargo

# Copy VERSION file for compile-time version injection (build.rs reads it)
COPY VERSION ./

COPY pdf-module-rs/crates ./crates

RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    if [ "$WITH_PDFIUM" = "1" ]; then \
        cargo build --profile "$BUILD_PROFILE" --bin pdf-mcp --bin pdf-web --bin pdf-cli --features pdf-core/knowledge,pdf-core/vlm; \
    else \
        cargo build --profile "$BUILD_PROFILE" --bin pdf-mcp --bin pdf-web --bin pdf-cli; \
    fi && \
    cp target/"$BUILD_PROFILE"/pdf-mcp /pdf-mcp && \
    cp target/"$BUILD_PROFILE"/pdf-web /pdf-web && \
    cp target/"$BUILD_PROFILE"/pdf-cli /pdf-cli 2>/dev/null || true

# ─── Stage 2a: Runtime — MCP stdio mode ────────────────────
FROM debian:bookworm-slim AS mcp

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1000 pdfuser

COPY --from=backend-builder /pdf-mcp /usr/local/bin/pdf-mcp
RUN chmod +x /usr/local/bin/pdf-mcp

USER pdfuser
ENV RUST_LOG=info

ENTRYPOINT ["pdf-mcp"]

# ─── Stage 2b: Runtime — Web server mode ───────────────────
FROM debian:bookworm-slim AS web

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1000 pdfuser

WORKDIR /app

COPY --from=backend-builder /pdf-web /usr/local/bin/pdf-web
RUN chmod +x /usr/local/bin/pdf-web && \
    mkdir -p /app/data /app/kb /app/logs /app/cache && \
    chown -R pdfuser:pdfuser /app

USER pdfuser

ENV RUST_LOG=info
ENV HTTP_PORT=8000
ENV KNOWLEDGE_BASE=/app/kb
ENV STORAGE_TYPE=local
ENV STORAGE_LOCAL_DIR=/app/data

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -sf http://localhost:8000/health || exit 1

CMD ["pdf-web"]

# ─── Stage 2c: Runtime — CLI mode ──────────────────────────
FROM debian:bookworm-slim AS cli

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1000 pdfuser

COPY --from=backend-builder /pdf-cli /usr/local/bin/pdf-cli
RUN chmod +x /usr/local/bin/pdf-cli

USER pdfuser

ENTRYPOINT ["pdf-cli"]

# ─── Stage 2d: Runtime — Full with PDFium ──────────────────
FROM debian:bookworm-slim AS full

ARG PDFIUM_VERSION=latest

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    wget \
    && rm -rf /var/lib/apt/lists/* && \
    useradd -m -u 1000 pdfuser

WORKDIR /app

# Download pdfium binaries for native PDF rendering
RUN wget -q -O /tmp/pdfium.tgz \
        "https://github.com/bblanchon/pdfium-binaries/releases/${PDFIUM_VERSION}/download/pdfium-linux-x64.tgz" && \
    tar -xzf /tmp/pdfium.tgz -C /usr/local && \
    ldconfig && \
    rm /tmp/pdfium.tgz

COPY --from=backend-builder /pdf-mcp /usr/local/bin/pdf-mcp
COPY --from=backend-builder /pdf-web /usr/local/bin/pdf-web
COPY --from=backend-builder /pdf-cli /usr/local/bin/pdf-cli 2>/dev/null || true

RUN chmod +x /usr/local/bin/pdf-mcp /usr/local/bin/pdf-web && \
    mkdir -p /app/data /app/kb /app/logs /app/cache && \
    chown -R pdfuser:pdfuser /app

USER pdfuser

ENV RUST_LOG=info
ENV PDFIUM_LIB_PATH=/usr/local/lib/libpdfium.so
ENV HTTP_PORT=8000
ENV KNOWLEDGE_BASE=/app/kb
ENV STORAGE_TYPE=local
ENV STORAGE_LOCAL_DIR=/app/data

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -sf http://localhost:8000/health || exit 1

CMD ["pdf-web"]