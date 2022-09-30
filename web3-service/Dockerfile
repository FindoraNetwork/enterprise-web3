FROM ghcr.io/findoranetwork/enterprise-web3:latest AS fundation
FROM docker.io/rust:slim
RUN apt-get update -y && apt-get install -y libssl-dev pkg-config make perl clang wget
RUN mkdir /web3-service
WORKDIR /web3-service
COPY --from=fundation /web3-service /web3-service/web3-service
RUN echo 'state_db_path = "/rocksdb-exporter/data/ledger/state.db"' >> /rocksdb-exporter/rocksdb-exporter-config.toml
RUN echo 'history_db_path = "/rocksdb-exporter/data/ledger/history.db"' >> /rocksdb-exporter/rocksdb-exporter-config.toml
RUN echo 'redis_url = ["redis://127.0.0.1:6379/0"]' >> /rocksdb-exporter/rocksdb-exporter-config.toml
RUN echo 'clear = true' >> /rocksdb-exporter/rocksdb-exporter-config.toml

ENTRYPOINT ["/web3-service/web3-service"]