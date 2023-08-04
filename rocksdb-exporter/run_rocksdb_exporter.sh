#!/bin/bash

URL=${LATEST_URL}
REDIS_HOST="${REDIS_HOST:=127.0.0.1}"
REDIS_PORT="${REDIS_PORT:=6379}"
ROOT_DIR=$(dirname `readlink -f $0`)
rm -f ${ROOT_DIR}/snapshot.tar.gz

if ! wget -O "${ROOT_DIR}/latest" "${URL}"; then
    echo "download latest failed, exit shell script"
    exit -1
fi

CHAINDATA_URL=$(cut -d , -f 1 "${ROOT_DIR}/latest")
rm ${ROOT_DIR}/latest -f

if ! wget -O "${ROOT_DIR}/snapshot/snapshot.tar.gz" "${CHAINDATA_URL}"; then
    echo "download latest failed, exit shell script"
    exit -1
fi

rm -rvf ${ROOT_DIR}/data/tendermint/mainnet/node0/data
rm -rvf ${ROOT_DIR}/data/ledger

tar -xvf ${ROOT_DIR}/snapshot/snapshot.tar.gz -C ${ROOT_DIR}

cat ${ROOT_DIR}/versioned-kv.lua | redis-cli -h ${REDIS_HOST} -p ${REDIS_PORT} -x FUNCTION LOAD REPLACE
sed -i "s#127.0.0.1#${REDIS_HOST}#g" ${ROOT_DIR}/rocksdb-exporter-config.toml
sed -i "s#6379#${REDIS_PORT}#g" ${ROOT_DIR}/rocksdb-exporter-config.toml

export EXPORT_CONFIG_FILE_PATH=${ROOT_DIR}/rocksdb-exporter-config.toml
${ROOT_DIR}/rocksdb-exporter
