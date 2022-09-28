#!/bin/bash

URL=${LATEST_URL}
ROOT_DIR=$(dirname `readlink -f $0`)
rm -f ${ROOT_DIR}/snapshot.tar.gz

if ! wget -O "${ROOT_DIR}/latest" "${URL}"; then
    echo "download latest failed, exit shell script"
    exit -1
fi

CHAINDATA_URL=$(cut -d , -f 1 "${ROOT_DIR}/latest")
rm ${ROOT_DIR}/latest -f

if ! wget -O "${ROOT_DIR}/snapshot.tar.gz" "${CHAINDATA_URL}"; then
    echo "download latest failed, exit shell script"
    exit -1
fi

rm -rvf ${ROOT_DIR}/data
tar -xvf ${ROOT_DIR}/snapshot.tar.gz


export EXPORT_CONFIG_FILE_PATH=${ROOT_DIR}/rocksdb-exporter-config.toml
${ROOT_DIR}/rocksdb-exporter
