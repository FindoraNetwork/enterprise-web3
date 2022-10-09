FROM ghcr.io/findoranetwork/enterprise-web3:latest AS fundation
FROM docker.io/rust:slim
RUN apt-get update -y && apt-get install -y libssl-dev pkg-config make perl clang wget
RUN mkdir /web3-service
WORKDIR /web3-service
COPY --from=fundation /web3-service /web3-service/web3-service

RUN echo 'http_port = 8545' >> /web3-service/web3-service-config.toml
RUN echo 'ws_port = 8546' >> /web3-service/web3-service-config.toml
RUN echo 'redis_url = ["redis://127.0.0.1:6379/0"]' >> /web3-service/web3-service-config.toml
RUN echo 'tendermint_url = "https://prod-mainnet.prod.findora.org:26657"' >> /web3-service/web3-service-config.toml
RUN echo 'chain_id = 2152' >> /web3-service/web3-service-config.toml
RUN echo 'gas_price = 10000000000' >> /web3-service/web3-service-config.toml

ENV WEB3_CONFIG_FILE_PATH=/web3-service/web3-service-config.toml
ENTRYPOINT ["/web3-service/web3-service"]