FROM rust:bullseye AS builder
WORKDIR /usr/local/app
COPY . .
RUN cargo install --path .


FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y cec-utils && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/hdmicec2mqtt /usr/local/bin/hdmicec2mqtt
WORKDIR /
CMD ["hdmicec2mqtt", "/config.toml"]
