FROM docker.io/rust:slim AS builder

RUN apt-get update -y && apt-get install -y libssl-dev pkg-config make perl clang
ENV OPENSSL_LIB_DIR="/usr/lib/x86_64-linux-gnu"
ENV OPENSSL_INCLUDE_DIR="/usr/include/openssl"
COPY . /enterprise-web3
WORKDIR /enterprise-web3
RUN cargo build --release

RUN mkdir /enterprise-web3-binaries
RUN cp target/release/rocksdb-exporter /enterprise-web3-binaries
RUN cp target/release/web3-service /enterprise-web3-binaries

RUN strip --strip-all /enterprise-web3-binaries/rocksdb-exporter
RUN strip --strip-all /enterprise-web3-binaries/web3-service

FROM scratch

COPY --from=builder /enterprise-web3-binaries /enterprise-web3-binaries
COPY redis-versioned-kv/lua/versioned-kv.lua /enterprise-web3-binaries/versioned-kv.lua
