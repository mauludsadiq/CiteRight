# Build stage
FROM rust:1.88-slim as builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy everything and build
COPY . .
RUN cargo build --release --features server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/citeright-server /usr/local/bin/citeright-server
COPY --from=builder /app/fixtures /app/fixtures
COPY --from=builder /app/templates /app/templates

ENV RUST_LOG=citeright=info
ENV CITERIGHT_FIXTURE=/app/fixtures/courtlistener_fixture.json

ENTRYPOINT ["/usr/local/bin/citeright-server"]
