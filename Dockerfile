FROM rust:latest as builder

RUN apt-get update && apt-get install -y musl-tools musl-dev && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --target x86_64-unknown-linux-musl --release

COPY src ./src
RUN touch src/main.rs
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:latest

COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/lightningnetworkrust /usr/local/bin/lightningnetworkrust

WORKDIR /app

EXPOSE 8080

CMD ["lightningnetworkrust"] 