FROM docker.io/library/rust:1.85.1-slim AS builder

WORKDIR /app
COPY Cargo.lock Cargo.toml ./
COPY knock/Cargo.toml knock/
COPY scripts/Cargo.toml scripts/
COPY knock/src/main.rs knock/src/
COPY scripts/src/main.rs scripts/src/
RUN cargo build -p knock --release || true
COPY knock knock
RUN cargo build -p knock --release

FROM docker.io/library/debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/knock /app/knock
COPY knock/default.env .
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
CMD ["./knock"]