FROM ghcr.io/findoranetwork/enterprise-web3:latest AS fundation
FROM docker.io/busybox:latest

RUN mkdir web3-service
COPY --from=fundation /web3-service /web3-service/web3-service
COPY --from=fundation /web3-service-config.toml /web3-service/web3-service-config.toml
ENV WEB3_CONFIG_FILE_PATH /web3-service/web3-service-config.toml
ENTRYPOINT ["/web3-service/web3-service"]
