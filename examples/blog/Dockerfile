# Build context: workspace root (axum-admin/)
# docker build -f examples/blog/Dockerfile -t blog .

# Build stage
FROM rust:1.88-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy entire workspace
COPY . .

# Build only the blog binary
RUN cargo build --release --manifest-path examples/blog/Cargo.toml

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/examples/blog/target/release/blog ./blog
COPY --from=builder /build/examples/blog/templates ./templates

EXPOSE 3000

CMD ["./blog"]
