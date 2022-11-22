FROM 358484141435.dkr.ecr.us-west-2.amazonaws.com/enterprise-web3:latest AS fundation
  
FROM debian:bullseye-slim
RUN apt-get update -y && apt-get install -y ca-certificates

RUN mkdir /web3-service
WORKDIR /web3-service

COPY --from=fundation /enterprise-web3-binaries/web3-service /web3-service/web3-service
COPY web3-service-config.toml /web3-service/web3-service-config.toml

EXPOSE 8545
EXPOSE 8546

ENV WEB3_CONFIG_FILE_PATH=/web3-service/web3-service-config.toml
CMD ["/web3-service/web3-service"]
