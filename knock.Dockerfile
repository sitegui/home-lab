FROM docker.io/library/rust:1.85.1-slim AS builder

WORKDIR /app
COPY Cargo.lock Cargo.toml ./
COPY home-lab/Cargo.toml home-lab/
COPY knock/Cargo.toml knock/
COPY scripts/Cargo.toml scripts/
RUN mkdir home-lab/src knock/src scripts/src
RUN touch home-lab/src/main.rs knock/src/main.rs scripts/src/main.rs
RUN cargo build -p knock --release || true
COPY knock knock
RUN cargo build -p knock --release

FROM docker.io/library/debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/knock /app/knock
COPY knock/default.env .
COPY knock/web/languages.json web/
CMD ["./knock"]