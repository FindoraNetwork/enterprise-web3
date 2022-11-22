FROM 358484141435.dkr.ecr.us-west-2.amazonaws.com/enterprise-web3:latest AS fundation

FROM debian:bullseye-slim
RUN apt-get update -y && apt-get install -y wget redis-tools

RUN mkdir /rocksdb-exporter
WORKDIR /rocksdb-exporter

COPY --from=fundation /enterprise-web3-binaries/rocksdb-exporter /rocksdb-exporter/rocksdb-exporter
COPY --from=fundation /enterprise-web3-binaries/versioned-kv.lua /rocksdb-exporter/versioned-kv.lua
COPY run_rocksdb_exporter.sh /rocksdb-exporter/run_rocksdb_exporter.sh
COPY rocksdb-exporter-config.toml /rocksdb-exporter/rocksdb-exporter-config.toml

ENV LATEST_URL "https://prod-mainnet-us-west-2-chain-data-backup.s3.us-west-2.amazonaws.com/latest"

RUN chmod +x /rocksdb-exporter/run_rocksdb_exporter.sh
CMD ["/rocksdb-exporter/run_rocksdb_exporter.sh"]
