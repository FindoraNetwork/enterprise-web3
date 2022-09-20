FROM ghcr.io/findoranetwork/enterprise-web3:latest AS fundation
FROM docker.io/busybox:latest

RUN mkdir rocksdb-exporter
COPY --from=fundation /rocksdb-exporter /rocksdb-exporter/rocksdb-exporter
COPY --from=fundation /rocksdb-exporter-config.toml /rocksdb-exporter/rocksdb-exporter-config.toml
ENV EXPORT_CONFIG_FILE_PATH /rocksdb-exporter/rocksdb-exporter-config.toml
ENTRYPOINT ["/rocksdb-exporter/rocksdb-exporter"]
