FROM rust:1.86-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin nautilus-sonar
RUN cargo build --release --bin consumer

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/nautilus-sonar /usr/local/bin/nautilus-sonar
COPY --from=builder /app/target/release/consumer /usr/local/bin/nautilus-consumer

CMD ["nautilus-sonar"]