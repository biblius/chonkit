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

RUN curl -sL \
  https://github.com/microsoft/onnxruntime/releases/download/v$ONX_VERSION/onnxruntime-linux-x64-gpu-$ONX_VERSION.tgz \
  | tar -xzf - -C ./onnxruntime

WORKDIR /app/feserver

RUN cargo build --release --target-dir ./target

FROM debian:latest

ARG ONX_VERSION

WORKDIR /app

COPY --from=builder /app/feserver/target/release/feserver ./feserver

# ONNX libraries
COPY --from=builder /app/onnxruntime/onnxruntime-linux-x64-${ONX_VERSION}/lib/libonnxruntime.so /usr/lib
COPY --from=builder /app/onnxruntime/onnxruntime-linux-x64-gpu-${ONX_VERSION}/lib/libonnxruntime.so /usr/lib

RUN apt-get update 
RUN apt-get install wget -y
RUN apt-get install software-properties-common -y

# https://developer.nvidia.com/cuda-downloads?target_os=Linux&target_arch=x86_64&Distribution=Debian&target_version=12&target_type=deb_local
RUN wget https://developer.download.nvidia.com/compute/cuda/12.6.3/local_installers/cuda-repo-debian12-12-6-local_12.6.3-560.35.05-1_amd64.deb
RUN dpkg -i cuda-repo-debian12-12-6-local_12.6.3-560.35.05-1_amd64.deb
RUN cp /var/cuda-repo-debian12-12-6-local/cuda-*-keyring.gpg /usr/share/keyrings/
RUN add-apt-repository contrib
RUN apt-get update

RUN apt-get -y install cuda-toolkit-12-6 
RUN apt-get install -y libssl3 

RUN apt clean 
RUN rm -rf /var/lib/apt/lists/*


EXPOSE 6969

ENTRYPOINT ["./feserver"]
