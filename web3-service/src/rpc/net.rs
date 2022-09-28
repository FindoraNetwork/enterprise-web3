use {
    jsonrpc_core::{futures::future, BoxFuture, Result},
    web3_rpc_core::{types::PeerCount, NetApi},
};

pub struct NetApiImpl;

impl NetApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl NetApi for NetApiImpl {
    fn version(&self) -> BoxFuture<Result<String>> {
        Box::pin(future::ok(String::from("1")))
    }

    fn peer_count(&self) -> BoxFuture<Result<PeerCount>> {
        Box::pin(async move { Ok(PeerCount::U32(0)) })
    }

    fn is_listening(&self) -> BoxFuture<Result<bool>> {
        Box::pin(future::ok(true))
    }
}
