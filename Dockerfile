FROM rust:latest AS builder

WORKDIR /app

ARG VEC_DB

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY sqlx-data.json ./sqlx-data.json

COPY src ./src

RUN cargo build --bin server -F "http fe-local ${VEC_DB}" --release

FROM debian:latest

WORKDIR /app

# Create upload directory
RUN mkdir upload

COPY --from=builder /app/target/release/server ./chonkit
COPY --from=builder /app/migrations ./migrations

ENTRYPOINT ["./chonkit"]

EXPOSE 42069

