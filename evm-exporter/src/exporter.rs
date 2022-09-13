use primitive_types::{H160, H256, U256};
use redis::{Commands, ConnectionLike};
use redis_versioned_kv::VersionedKVCommand;

use crate::{keys, AccountBasic, Block, Result, Transaction};

pub struct Exporter<C> {
    conn: C,
    pub prefix: String,
    pub height: u32,
}

impl<C: ConnectionLike> Exporter<C> {
    pub fn new(conn: C, prefix: String) -> Self {
        Self {
            conn,
            prefix,
            height: 0,
        }
    }

    pub fn begin_block(&mut self, height: u32, _block: Block) -> Result<()> {
        self.height = height;
        Ok(())
    }

    pub fn end_block(&mut self, _block: Block) -> Result<()> {
        // Set current key here.
        let height_key = keys::latest_height_key(&self.prefix);

        self.conn.set(height_key, format!("{}", self.height))?;

        Ok(())
    }

    pub fn begin_transaction(&mut self, _hash: Vec<u8>, _tx: Transaction) -> Result<()> {
        Ok(())
    }

    pub fn end_transaction(&mut self, _hash: Vec<u8>, _tx: Transaction) -> Result<()> {
        Ok(())
    }

    pub fn add_receipt(&mut self, _txhash: Vec<u8>, _tx: Transaction) -> Result<()> {
        Ok(())
    }

    pub fn update_basic(&mut self, address: H160, basic: AccountBasic) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        let code_key = keys::code_key(&self.prefix, address);
        let nonce_key = keys::nonce_key(&self.prefix, address);

        let height = self.height;

        self.conn
            .vkv_set(balance_key, height, keys::hex_u256(basic.balance))?;
        self.conn
            .vkv_set(code_key, height, hex::encode(basic.code))?;
        self.conn
            .vkv_set(nonce_key, height, keys::hex_u256(basic.nonce))?;

        Ok(())
    }

    pub fn update_state(&mut self, address: H160, index: H256, value: H256) -> Result<()> {
        let state_key = keys::state_key(&self.prefix, address, index);

        if value.is_zero() {
            self.conn.vkv_del(state_key, self.height)?;
        } else {
            self.conn
                .vkv_set(state_key, self.height, hex::encode(value))?;
        }

        Ok(())
    }
}
