FROM rust:1.86-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build Nautilus sonar
RUN cargo build --release --bin nautilus-sonar

# Build consumer
RUN cargo build --release --bin consumer

# Build correlator
RUN cargo build --release --bin correlator


FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*


COPY --from=builder /app/target/release/nautilus-sonar /usr/local/bin/nautilus-sonar

COPY --from=builder /app/target/release/consumer /usr/local/bin/nautilus-consumer

COPY --from=builder /app/target/release/correlator /usr/local/bin/nautilus-correlator


CMD ["nautilus-sonar"]