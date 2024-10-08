use crate::vm::precompile::GETTER;

mod config;
mod notify;
mod rpc;
mod utils;
mod vm;

#[cfg(feature = "redis")]
use evm_exporter::{RedisGetter, PREFIX};

#[cfg(feature = "redis-cluster")]
use evm_exporter::{RedisClusterGetter, PREFIX};

#[cfg(feature = "postgres")]
use evm_exporter::PgGetter;

use {
    config::Config,
    evm_exporter::{ConnectionType, Getter},
    jsonrpc_core::MetaIoHandler,
    jsonrpc_http_server::DomainsValidation,
    jsonrpc_pubsub::PubSubHandler,
    notify::SubscriberNotify,
    rpc::{
        debug::DebugApiImpl,
        debugapi::{debug::DebugApi, jsvm::params::init_upstream},
        eth::EthService,
        eth_filter::EthFilterApiImpl,
        eth_pubsub::EthPubSubApiImpl,
        health::{HealthApi, HealthApiImpl},
        net::NetApiImpl,
        web3::Web3ApiImpl,
    },
    ruc::*,
    std::{
        net::SocketAddr,
        sync::Arc,
        thread::{self, available_parallelism},
    },
    tendermint_rpc::HttpClient,
    web3_rpc_core::{EthApi, EthFilterApi, EthPubSubApi, NetApi, Web3Api},
};

fn main() {
    env_logger::init();
    let config_path = pnk!(std::env::var("WEB3_CONFIG_FILE_PATH"));
    let config = pnk!(Config::new(&config_path));

    let http = format!("0.0.0.0:{}", config.http_port);
    let ws = format!("0.0.0.0:{}", config.ws_port);

    #[cfg(feature = "redis")]
    let getter: Arc<dyn Getter + Sync + Send> = Arc::new(RedisGetter::new(
        ConnectionType::Redis(config.redis_url[0].clone()),
        PREFIX.to_string(),
    ));

    #[cfg(feature = "redis-cluster")]
    let getter: Arc<dyn Getter + Sync + Send> = Arc::new(RedisGetter::new(
        ConnectionType::RedisCluster(config.redis_url.clone()),
        PREFIX.to_string(),
    ));

    #[cfg(feature = "postgres")]
    let getter: Arc<dyn Getter + Sync + Send> = Arc::new(PgGetter::new(
        ConnectionType::Postgres(config.postgres_uri),
        String::new(),
    ));

    // pnk!(GETTER.set(getter.clone()).map_err(|_| eg!()));
    pnk!(init_upstream(getter.clone()));

    let tm_client = Arc::new(pnk!(HttpClient::new(config.tendermint_url.as_str())));
    let eth = EthService::new(
        config.chain_id,
        config.gas_price,
        getter.clone(),
        tm_client,
        config.tendermint_url.as_str(),
    );
    let net = NetApiImpl::new();
    let web3 = Web3ApiImpl::new();
    let debug = DebugApiImpl::new(
        config.chain_id,
        config.gas_price,
        getter.clone(),
        config.tendermint_url.as_str(),
    );
    let health = HealthApiImpl::new();
    let filter = EthFilterApiImpl::new(getter.clone());
    let subscriber_notify = Arc::new(SubscriberNotify::new(
        getter.clone(),
        &config.tendermint_url,
    ));
    pnk!(subscriber_notify.start());
    let pub_sub = EthPubSubApiImpl::new(getter.clone(), subscriber_notify);

    let mut io = MetaIoHandler::default();
    io.extend_with(eth.to_delegate());
    io.extend_with(net.to_delegate());
    io.extend_with(web3.to_delegate());
    io.extend_with(debug.to_delegate());
    io.extend_with(health.to_delegate());
    io.extend_with(filter.to_delegate());
    let mut io = PubSubHandler::new(io);
    io.extend_with(pub_sub.to_delegate());

    let http_addr = pnk!(http.parse::<SocketAddr>());
    let http_server = jsonrpc_http_server::ServerBuilder::new(io.clone())
        .health_api(("/health", "system_health"))
        .threads(
            available_parallelism()
                .map(usize::from)
                .unwrap_or_else(|_| num_cpus::get()),
        )
        .keep_alive(true)
        .start_http(&http_addr)
        .expect("failed to create http server");
    thread::spawn(move || {
        let ws_addr = pnk!(ws.parse::<SocketAddr>());
        let ws_server = jsonrpc_ws_server::ServerBuilder::with_meta_extractor(
            io,
            |context: &jsonrpc_ws_server::RequestContext| context.sender().into(),
        )
        .max_payload(15 * 1024 * 1024)
        .max_connections(100)
        .allowed_origins(map_cors(Some(&vec!["*".to_string()])))
        .start(&ws_addr)
        .expect("failed to create ws server");
        println!("*** Web3-websocket serve at {} ***", ws);
        pnk!(ws_server.wait());
    });
    println!("*** Web3-http serve at {} ***", http);
    http_server.wait();
}

fn map_cors<T: for<'a> From<&'a str>>(cors: Option<&Vec<String>>) -> DomainsValidation<T> {
    cors.map(|x| {
        x.iter()
            .map(AsRef::as_ref)
            .map(Into::into)
            .collect::<Vec<_>>()
    })
    .into()
}
