ARG ONX_VERSION=1.20.1

FROM rust:latest AS builder

ARG ONX_VERSION

WORKDIR /app

COPY feserver ./feserver
COPY embedders ./embedders

RUN mkdir onnxruntime

RUN curl -sL \
  https://github.com/microsoft/onnxruntime/releases/download/v$ONX_VERSION/onnxruntime-linux-x64-$ONX_VERSION.tgz \
  | tar -xzf - -C ./onnxruntime

WORKDIR /app/feserver

RUN cargo build --release --target-dir ./target

FROM debian:latest

ARG ONX_VERSION

WORKDIR /app

COPY --from=builder /app/feserver/target/release/feserver ./feserver
COPY --from=builder /app/onnxruntime/onnxruntime-linux-x64-${ONX_VERSION}/lib/libonnxruntime.so /usr/lib

RUN apt-get update && apt-get install -y libssl3 && apt clean && rm -rf /var/lib/apt/lists/*

EXPOSE 6969

ENTRYPOINT ["./feserver"]
