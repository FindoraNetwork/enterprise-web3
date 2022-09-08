use ethereum_types::{H160, H256, U256};
use evm::backend::{Backend, Basic};
use redis::{
    ConnectionLike, Client
};
use anyhow::*;

#[derive(Clone, Debug)]
pub struct EthVmBackend<C>  {
    conn: C,
    gas_price: U256,
}

impl<C: ConnectionLike> EthVmBackend<C> {
    pub fn new(redis_addr: &str, gas_price: u64,) -> Result<Self> {
        let con = Client::open(redis_addr)?;
        Ok(Self{
            conn: con,
            gas_price: U256::from(gas_price),
        })
    }
}

impl<C: ConnectionLike> Backend for EthVmBackend<C> {
    fn gas_price(&self) -> U256 {
        self.gas_price
    }

    fn origin(&self) -> H160 {
        todo!()
    }

    fn block_hash(&self, number: U256) -> H256 {
        todo!()
    }

    fn block_number(&self) -> U256 {
        todo!()
    }

    fn block_coinbase(&self) -> H160 {
        todo!()
    }

    fn block_timestamp(&self) -> U256 {
        todo!()
    }

    fn block_difficulty(&self) -> U256 {
        todo!()
    }

    fn block_gas_limit(&self) -> U256 {
        todo!()
    }

    fn chain_id(&self) -> U256 {
        todo!()
    }

    fn exists(&self, address: H160) -> bool {
        todo!()
    }

    fn basic(&self, address: H160) -> Basic {
        todo!()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        todo!()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        todo!()
    }

    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        todo!()
    }
}