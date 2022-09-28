use {
    ethereum_types::H256,
    jsonrpc_core::{futures::future, BoxFuture, Result},
    sha3::{Digest, Keccak256},
    web3_rpc_core::{types::Bytes, Web3Api},
};

pub struct Web3ApiImpl;

impl Web3ApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl Web3Api for Web3ApiImpl {
    fn client_version(&self) -> BoxFuture<Result<String>> {
        Box::pin(future::ok(String::from("1")))
    }

    fn sha3(&self, input: Bytes) -> Result<H256> {
        Ok(H256::from_slice(
            Keccak256::digest(&input.into_vec()).as_slice(),
        ))
    }
}
