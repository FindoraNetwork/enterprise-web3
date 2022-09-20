FROM docker.io/rust:slim AS builder

RUN apt-get update -y && apt-get install -y libssl-dev pkg-config make perl clang
ENV OPENSSL_LIB_DIR="/usr/lib/x86_64-linux-gnu"
ENV OPENSSL_INCLUDE_DIR="/usr/include/openssl"
RUN rustup target add x86_64-unknown-linux-musl
COPY . ./enterprise-web3
WORKDIR /enterprise-web3
RUN cargo build --release

RUN mkdir /enterprise-web3-binaries
RUN cp rocksdb-exporter/rocksdb-exporter-config.toml /enterprise-web3-binaries
RUN cp target/release/rocksdb-exporter /enterprise-web3-binaries
RUN cp web3-service/rocksdb-exporter-config.toml /enterprise-web3-binaries
RUN cp target/release/web3-service /enterprise-web3-binaries
RUN strip --strip-all /enterprise-web3-binaries/rocksdb-exporter
RUN strip --strip-all /enterprise-web3-binaries/web3-service

FROM docker.io/busybox:latest

COPY --from=builder /enterprise-web3-binaries/web3-service /web3-service
COPY --from=builder /enterprise-web3-binaries/web3-service-config.toml /web3-service-config.toml
COPY --from=builder /enterprise-web3-binaries/rocksdb-exporter /rocksdb-exporter
COPY --from=builder /enterprise-web3-binaries/rocksdb-exporter-config.toml /rocksdb-exporter-config.toml
