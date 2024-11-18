FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY sqlx-data.json ./sqlx-data.json
COPY src ./src

RUN mkdir pdfium && curl -sL https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F6666/pdfium-linux-x64.tgz | tar -xzf - -C ./pdfium
RUN mkdir onnxruntime && curl -sL https://github.com/microsoft/onnxruntime/releases/download/v1.19.2/onnxruntime-linux-x64-1.19.2.tgz | tar -xzf - -C ./onnxruntime

RUN cargo build --bin server --no-default-features -F "fe-local qdrant weaviate" --release

FROM debian:latest

WORKDIR /app

# Create upload directory
RUN mkdir upload

COPY --from=builder /app/target/release/server ./chonkit
COPY --from=builder /app/migrations ./migrations
COPY --from=builder /app/pdfium/lib/libpdfium.so /usr/lib
COPY --from=builder /app/onnxruntime/onnxruntime-linux-x64-1.19.2/lib/libonnxruntime.so /usr/lib

RUN apt-get update && apt-get install -y libssl3 && apt clean && rm -rf /var/lib/apt/lists/*

RUN useradd -Mr rust
RUN chown -R rust: /app

EXPOSE 42069

USER rust

CMD ./chonkit -l debug
