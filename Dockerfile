FROM rust:latest as builder

# Install build dependencies for duckdb and sled (requires cc, pkg-config, openssl, etc)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build release binary
RUN cargo build --release

# Create a lightweight runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/evento /usr/local/bin/evento

# Ensure storage directory exists and has correct permissions
RUN mkdir -p /root/.evento/data
VOLUME /root/.evento/data

ENTRYPOINT ["evento"]
