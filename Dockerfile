FROM rust:bookworm AS backend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY pdf-module-rs/Cargo.toml pdf-module-rs/Cargo.lock ./
COPY pdf-module-rs/crates ./crates
COPY pdf-module-rs/.cargo ./.cargo

RUN cargo build --release --bin pdf-mcp --bin pdf-dashboard

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 pdfuser

WORKDIR /app

COPY --from=backend-builder /app/target/release/pdf-mcp /usr/local/bin/pdf-mcp
COPY --from=backend-builder /app/target/release/pdf-dashboard /usr/local/bin/pdf-dashboard

RUN chmod +x /usr/local/bin/pdf-mcp /usr/local/bin/pdf-dashboard && \
    mkdir -p /app/data /app/logs/audit /app/cache && \
    chown -R pdfuser:pdfuser /app

USER pdfuser

ENV RUST_LOG=info
ENV STORAGE_TYPE=local
ENV STORAGE_LOCAL_DIR=/app/data
ENV CACHE_ENABLED=true
ENV CACHE_MAX_SIZE=1000
ENV AUDIT_ENABLED=true
ENV AUDIT_LOG_DIR=/app/logs/audit
ENV MAX_FILE_SIZE_MB=100
ENV DASHBOARD_PORT=8000

EXPOSE 8000 8001

CMD ["pdf-dashboard"]
