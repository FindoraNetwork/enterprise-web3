use primitive_types::{H160, U256, H256};
use redis::ConnectionLike;

mod error;
pub use error::*;

mod types;
use redis_versioned_kv::VersionedKVCommand;
pub use types::*;

pub mod keys;

pub struct Exporter<C> {
    conn: C,
    pub prefix: String,
}

impl<C: ConnectionLike> Exporter<C> {
    pub fn new(conn: C, prefix: String) -> Self {
        Self { conn, prefix }
    }

    pub fn add_block(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn add_transaction(&mut self, _hash: Vec<u8>) -> Result<()> {
        Ok(())
    }

    pub fn add_receipt(&mut self, _txhash: Vec<u8>) -> Result<()> {
        Ok(())
    }

    pub fn update_basic(&mut self, address: H160, height: u32, basic: AccountBasic) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        let code_key = keys::code_key(&self.prefix, address);
        let nonce_key = keys::nonce_key(&self.prefix, address);

        self.conn.vkv_set(balance_key, height, keys::hex_u256(basic.balance))?;
        self.conn.vkv_set(code_key, height, hex::encode(basic.code))?;
        self.conn.vkv_set(nonce_key, height, keys::hex_u256(basic.nonce))?;

        Ok(())
    }

    pub fn update_state(&mut self, address: H160, index: U256, value: H256) -> Result<()> {
        let state_key = keys::state_key(&self.prefix, address, index);

        Ok(())
    }
}
