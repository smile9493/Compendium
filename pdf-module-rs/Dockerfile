# Stage 1: Build
FROM rust:1.85-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/pdf-core ./crates/pdf-core
COPY crates/pdf-rest ./crates/pdf-rest
COPY crates/pdf-mcp ./crates/pdf-mcp
COPY crates/pdf-python ./crates/pdf-python

# Build release binaries
RUN cargo build --release -p pdf-rest

# Stage 2: Runtime (REST mode)
FROM debian:bookworm-slim AS rest

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/pdf-rest /usr/local/bin/pdf-rest

# Create non-root user
RUN useradd -r -s /bin/false pdfuser

# Set environment defaults
ENV PDF_SERVER_HOST=0.0.0.0
ENV PDF_SERVER_PORT=8000
ENV PDF_DEFAULT_ADAPTER=lopdf
ENV PDF_CACHE_ENABLED=true
ENV PDF_MAX_FILE_SIZE_MB=200

# Expose port
EXPOSE 8000

# Switch to non-root user
USER pdfuser

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/api/v1/x2text/health || exit 1

# Entry point
ENTRYPOINT ["pdf-rest"]
