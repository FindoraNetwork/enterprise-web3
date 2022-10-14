mod config;
mod rpc;
mod utils;
mod vm;

use {
    crate::rpc::health::{HealthApi, HealthApiImpl},
    config::Config,
    jsonrpc_core::IoHandler,
    rpc::{eth::EthService, net::NetApiImpl, web3::Web3ApiImpl},
    ruc::*,
    std::{
        net::SocketAddr,
        sync::Arc,
        thread::{self, available_parallelism},
    },
    web3_rpc_core::{EthApi, NetApi, Web3Api},
};

fn main() {
    env_logger::init();
    let config_path = pnk!(std::env::var("WEB3_CONFIG_FILE_PATH"));
    let config = pnk!(Config::new(&config_path));

    let http = format!("0.0.0.0:{}", config.http_port);
    let ws = format!("0.0.0.0:{}", config.ws_port);
    #[cfg(feature = "cluster_redis")]
    let client = pnk!(redis::cluster::ClusterClient::open(
        config.redis_url.clone()
    ));
    #[cfg(not(feature = "cluster_redis"))]
    let client = pnk!(redis::Client::open(config.redis_url[0].as_ref()));

    let pool = Arc::new(pnk!(r2d2::Pool::builder().max_size(50).build(client)));

    let tendermint_rpc = config.tendermint_url;

    let mut io = IoHandler::new();
    let eth = EthService::new(
        config.chain_id,
        config.gas_price,
        pool.clone(),
        tendermint_rpc.as_str(),
    );

    let net = NetApiImpl::new();
    let web3 = Web3ApiImpl::new();
    let health = HealthApiImpl::new();

    io.extend_with(eth.to_delegate());
    io.extend_with(net.to_delegate());
    io.extend_with(web3.to_delegate());
    io.extend_with(health.to_delegate());

    let http_addr = pnk!(http.parse::<SocketAddr>());
    let http_server = jsonrpc_http_server::ServerBuilder::new(io.clone())
        .health_api(("/health", "system_health"))
        .threads(
            available_parallelism()
                .map(usize::from)
                .unwrap_or(num_cpus::get()),
        )
        .keep_alive(true)
        .start_http(&http_addr)
        .expect("failed to create http server");
    thread::spawn(move || {
        let ws_addr = pnk!(ws.parse::<SocketAddr>());
        let ws_server = jsonrpc_ws_server::ServerBuilder::new(io)
            .start(&ws_addr)
            .expect("failed to create ws server");
        println!("*** Web3-websocket serve at {} ***", ws);
        pnk!(ws_server.wait());
    });
    println!("*** Web3-http serve at {} ***", http);
    http_server.wait();
}
