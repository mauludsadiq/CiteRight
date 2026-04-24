# Build stage
FROM rust:1.88-slim as builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/server.rs && \
    echo "pub mod claims; pub mod courtlistener; pub mod document; pub mod emit; pub mod extract; pub mod hash; pub mod models; pub mod planner; pub mod candidates; pub mod artifact; pub mod snapshot; pub mod bindings; pub mod audit; pub mod verify_selected; pub mod report; pub mod reasoning; pub mod resolver; pub mod selector; pub mod verify;" > src/lib.rs && \
    mkdir -p src/reasoning && \
    touch src/reasoning/mod.rs src/claims.rs src/courtlistener.rs src/document.rs src/emit.rs src/extract.rs src/hash.rs src/models.rs src/planner.rs src/candidates.rs src/artifact.rs src/snapshot.rs src/bindings.rs src/audit.rs src/verify_selected.rs src/report.rs src/resolver.rs src/selector.rs src/verify.rs
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

ENTRYPOINT ["/bin/sh", "-c", "echo Starting on port $PORT && /usr/local/bin/citeright-server"]
