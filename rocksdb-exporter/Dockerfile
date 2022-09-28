FROM ghcr.io/findoranetwork/enterprise-web3:latest AS fundation
FROM docker.io/busybox:latest

RUN mkdir rocksdb-exporter
COPY --from=fundation /rocksdb-exporter /rocksdb-exporter/rocksdb-exporter
COPY --from=fundation /run_rocksdb_exporter.sh /rocksdb-exporter/run_rocksdb_exporter.sh
COPY --from=fundation /rocksdb-exporter-config.toml /rocksdb-exporter/rocksdb-exporter-config.toml


ENV LATEST_URL "https://prod-mainnet-us-west-2-chain-data-backup.s3.us-west-2.amazonaws.com/latest"

ENTRYPOINT ["/rocksdb-exporter/run_rocksdb_exporter.sh"]
