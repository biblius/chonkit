FROM rust:latest as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY .sqlx ./.sqlx

COPY src ./src

RUN cargo build --release

FROM debian:latest

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY config.json ./config.json
COPY content ./content
COPY --from=builder /app/target/release/chonkit ./chonkit
COPY --from=builder /app/migrations ./migrations

ENTRYPOINT ["./chonkit"]

EXPOSE 42069

