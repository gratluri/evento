FROM rust:latest as builder

WORKDIR /app
ARG CACHE_BUST=4
RUN apt-get update && apt-get install -y lld clang && rm -rf /var/lib/apt/lists/*
COPY . /app

# Build debug binary and examples for validation with restricted parallelism and lld linker
ENV RUSTFLAGS="-C link-arg=-fuse-ld=lld"
RUN cargo build -j 1
RUN cargo build --examples -j 1

# Create a lightweight runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy validation example from builder
COPY --from=builder /app/target/debug/examples/validate_phase1 /usr/local/bin/validate_phase1
# Copy the main binary
COPY --from=builder /app/target/debug/evento /usr/local/bin/evento
# Copy the client binary
COPY --from=builder /app/target/debug/evento-client /usr/local/bin/evento-client

# Ensure storage directory exists and has correct permissions
RUN mkdir -p /root/.evento/data
VOLUME /root/.evento/data

# Expose the Admin UI port and Simulator port
EXPOSE 8080 8081

ENTRYPOINT ["evento"]
CMD ["server", "--port", "8080"]
