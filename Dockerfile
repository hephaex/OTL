# OTL API Server Dockerfile
# Multi-stage build for optimal image size
#
# Author: hephaex@gmail.com

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM rust:1.84-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY crates/otl-core/Cargo.toml crates/otl-core/
COPY crates/otl-parser/Cargo.toml crates/otl-parser/
COPY crates/otl-ocr/Cargo.toml crates/otl-ocr/
COPY crates/otl-graph/Cargo.toml crates/otl-graph/
COPY crates/otl-vector/Cargo.toml crates/otl-vector/
COPY crates/otl-extractor/Cargo.toml crates/otl-extractor/
COPY crates/otl-rag/Cargo.toml crates/otl-rag/
COPY crates/otl-api/Cargo.toml crates/otl-api/
COPY crates/otl-cli/Cargo.toml crates/otl-cli/

# Create dummy source files for dependency caching
RUN mkdir -p crates/otl-core/src && echo "pub fn dummy() {}" > crates/otl-core/src/lib.rs
RUN mkdir -p crates/otl-parser/src && echo "pub fn dummy() {}" > crates/otl-parser/src/lib.rs
RUN mkdir -p crates/otl-ocr/src && echo "pub fn dummy() {}" > crates/otl-ocr/src/lib.rs
RUN mkdir -p crates/otl-graph/src && echo "pub fn dummy() {}" > crates/otl-graph/src/lib.rs
RUN mkdir -p crates/otl-vector/src && echo "pub fn dummy() {}" > crates/otl-vector/src/lib.rs
RUN mkdir -p crates/otl-extractor/src && echo "pub fn dummy() {}" > crates/otl-extractor/src/lib.rs
RUN mkdir -p crates/otl-rag/src && echo "pub fn dummy() {}" > crates/otl-rag/src/lib.rs
RUN mkdir -p crates/otl-api/src && echo "pub fn dummy() {}" > crates/otl-api/src/lib.rs && echo "fn main() {}" > crates/otl-api/src/main.rs
RUN mkdir -p crates/otl-cli/src && echo "fn main() {}" > crates/otl-cli/src/main.rs

# Build dependencies only (for caching)
RUN cargo build --release -p otl-api 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Touch the source files to invalidate cache
RUN touch crates/*/src/*.rs

# Build the actual application
RUN cargo build --release -p otl-api

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    tesseract-ocr \
    tesseract-ocr-kor \
    tesseract-ocr-eng \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false otl

# Copy binary from builder
COPY --from=builder /app/target/release/otl-api /usr/local/bin/otl-api

# Set ownership
RUN chown otl:otl /usr/local/bin/otl-api

# Switch to non-root user
USER otl

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Environment variables
ENV API_HOST=0.0.0.0
ENV API_PORT=8080
ENV RUST_LOG=otl_api=info,tower_http=info

# Run the application
ENTRYPOINT ["/usr/local/bin/otl-api"]
