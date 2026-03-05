# Build stage
FROM rust:1.85-slim-bookworm as builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*


# Copy the entire workspace
COPY Cargo.toml Cargo.lock ./
COPY relay-common ./relay-common
COPY relay-gateway ./relay-gateway
COPY relay-agent ./relay-agent

# Build only the gateway
RUN cargo build --release -p relay-gateway

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (like SSL certs if needed for external requests, even though we offload incoming TLS)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/app/target/release/relay-gateway /usr/local/bin/

# Expose WS and HTTP ports
EXPOSE 8080

# Environment variables expected at runtime (can be overridden)
ENV RELAY_AGENT_TOKEN=""
ENV RELAY_API_TOKEN=""
ENV RELAY_API_PORT="8080"

ENTRYPOINT ["relay-gateway"]
