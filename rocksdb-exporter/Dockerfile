FROM ghcr.io/findoranetwork/enterprise-web3:latest AS fundation

FROM docker.io/rust:slim
RUN apt-get update -y && apt-get install -y libssl-dev pkg-config make perl clang wget
RUN mkdir /rocksdb-exporter
WORKDIR /rocksdb-exporter
RUN echo 'state_db_path = "/rocksdb-exporter/data/ledger/state.db"' >> /rocksdb-exporter/rocksdb-exporter-config.toml
RUN echo 'history_db_path = "/rocksdb-exporter/data/ledger/history.db"' >> /rocksdb-exporter/rocksdb-exporter-config.toml
RUN echo 'redis_url = ["redis://127.0.0.1:6379/0"]' >> /rocksdb-exporter/rocksdb-exporter-config.toml
RUN echo 'clear = true' >> /rocksdb-exporter/rocksdb-exporter-config.toml

COPY --from=builder /enterprise-web3-binaries/rocksdb-exporter /rocksdb-exporter/rocksdb-exporter
COPY --from=builder /enterprise-web3-binaries/run_rocksdb_exporter.sh /rocksdb-exporter/run_rocksdb_exporter.sh

ENV LATEST_URL "https://prod-mainnet-us-west-2-chain-data-backup.s3.us-west-2.amazonaws.com/latest"

ENTRYPOINT ["/rocksdb-exporter/run_rocksdb_exporter.sh"]
