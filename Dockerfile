FROM rust:latest as builder

WORKDIR /app
ARG CACHE_BUST=4
COPY . /app

# Build release binary and examples for validation
RUN cargo build --release
RUN cargo build --release --examples

# Create a lightweight runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy validation example from builder
COPY --from=builder /app/target/release/examples/validate_phase1 /usr/local/bin/validate_phase1
# Copy the main binary
COPY --from=builder /app/target/release/evento /usr/local/bin/evento

# Ensure storage directory exists and has correct permissions
RUN mkdir -p /root/.evento/data
VOLUME /root/.evento/data

# Expose the Admin UI port
EXPOSE 8080

ENTRYPOINT ["evento"]
