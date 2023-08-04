docker run -v /data:/rocksdb-exporter/data \
           -v /tmp/snapshot:/rocksdb-exporter/snapshot \
	   -e LATEST_URL=https://prod-testnet-us-west-2-chain-data-backup.s3.us-west-2.amazonaws.com/latest \
	   -e REDIS_HOST=35.93.22.13 -e REDIS_PORT=9999 \
	   --rm ghcr.io/findoranetwork/enterprise-web3:rocksdb-exporter-latest

