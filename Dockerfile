# Build stage
FROM rust:1.76-slim as builder

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Build actual binary
COPY src ./src
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/citeright /usr/local/bin/citeright

# Create directories for input/output
RUN mkdir -p /app/input /app/output /app/fixtures

VOLUME ["/app/input", "/app/output", "/app/fixtures"]

ENV RUST_LOG=citeright=info

ENTRYPOINT ["citeright"]
CMD ["--help"]
