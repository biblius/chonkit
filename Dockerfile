FROM rust:latest AS builder

WORKDIR /app

ARG VEC_DB

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY .sqlx ./.sqlx

COPY src ./src

RUN cargo build -F http -F ${VEC_DB} --release

FROM debian:latest

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create upload directory

RUN mkdir upload
COPY --from=builder /app/target/release/chonkit ./chonkit
COPY --from=builder /app/migrations ./migrations

ENTRYPOINT ["./chonkit"]

EXPOSE 42069

