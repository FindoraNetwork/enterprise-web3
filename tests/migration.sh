#!/bin/bash

apt update && apt install wget -y

wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/allowances.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f allowances.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/balance.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f balance.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/block_info.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f block_info.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/byte_code.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f byte_code.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/common.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f common.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/issuance.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f issuance.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/nonce.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f nonce.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/pending_byte_code.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f pending_byte_code.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/pending_state.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f pending_state.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/pending_transactions.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f pending_transactions.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/state.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f state.sql
wget https://raw.githubusercontent.com/FindoraNetwork/enterprise-web3/main/evm-exporter/migrations/transactions.sql && psql "postgresql://postgres:mysecretpassword@postgres:5432/mydatabase" -f transactions.sql
