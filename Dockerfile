# Build stage
FROM rust:1.85-slim as builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --features server
RUN rm -rf src

# Build actual binaries
COPY src ./src
COPY templates ./templates
RUN touch src/main.rs && cargo build --release --features server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/citeright /usr/local/bin/citeright
COPY --from=builder /app/target/release/citeright-server /usr/local/bin/citeright-server
COPY --from=builder /app/templates /app/templates
COPY fixtures/ /app/fixtures/

ENV RUST_LOG=citeright=info
ENV CITERIGHT_FIXTURE=/app/fixtures/courtlistener_fixture.json

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/citeright-server"]
