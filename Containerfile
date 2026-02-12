# RegelRecht Upload Portal - Multi-stage Container Build
# Build with: podman build -f Containerfile -t regelrecht-upload .

# =============================================================================
# Build Stage
# =============================================================================
FROM docker.io/library/rust:1.85-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy src and pin transitive dependencies to Rust 1.85-compatible versions
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo update home --precise 0.5.9 && \
    cargo update getrandom@0.2 --precise 0.2.15 && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN touch src/main.rs && \
    cargo build --release

# =============================================================================
# Runtime Stage
# =============================================================================
FROM docker.io/library/debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false appuser

# Copy binary from builder
COPY --from=builder /app/target/release/regelrecht-upload /app/regelrecht-upload

# Copy frontend assets
COPY frontend /app/frontend

# Create upload directory
RUN mkdir -p /app/uploads && chown -R appuser:appuser /app

# Set environment defaults
ENV HOST=0.0.0.0
ENV PORT=8080
ENV FRONTEND_DIR=/app/frontend
ENV UPLOAD_DIR=/app/uploads
ENV RUST_LOG=info

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/faq || exit 1

# Run the application
CMD ["/app/regelrecht-upload"]
