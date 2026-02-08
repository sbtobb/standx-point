FROM rust:stable AS builder

WORKDIR /app
COPY . .

RUN cargo build -p standx-point-mm-strategy --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/standx-point-mm-strategy /usr/local/bin/standx-point-mm-strategy

ENV RUST_LOG=info

ENTRYPOINT ["standx-point-mm-strategy", "--env"]
