use {
    crate::{
        error::{Error, Result},
        keys,
        types::{Block, TransactionStatus},
    },
    ethereum::FrontierReceiptData,
    primitive_types::{H160, H256, U256},
    redis::{Commands, ConnectionLike},
    redis_versioned_kv::VersionedKVCommand,
};

pub struct Setter<C> {
    conn: C,
    pub prefix: String,
}

impl<C: ConnectionLike> Setter<C> {
    pub fn new(conn: C, prefix: String) -> Self {
        Self { conn, prefix }
    }

    pub fn set_height(&mut self, height: u32) -> Result<()> {
        let height_key = keys::latest_height_key(&self.prefix);
        self.conn.set(height_key, format!("{}", height))?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        redis::cmd("FLUSHALL")
            .arg("SYNC")
            .query(&mut self.conn)
            .map_err(|e| Error::RedisError(e))?;
        Ok(())
    }

    pub fn set_account_basic(
        &mut self,
        height: u32,
        basic: Vec<(H160, (U256, U256))>,
    ) -> Result<()> {
        for (address, (nonce, balance)) in basic {
            let balance_key = keys::balance_key(&self.prefix, address);
            let nonce_key = keys::nonce_key(&self.prefix, address);
            self.conn
                .vkv_set(balance_key, height, serde_json::to_string(&balance)?)
                .map_err(|e| Error::RedisError(e))?;

            self.conn
                .vkv_set(nonce_key, height, serde_json::to_string(&nonce)?)
                .map_err(|e| Error::RedisError(e))?;
        }
        Ok(())
    }

    pub fn set_codes(&mut self, height: u32, codes: Vec<(H160, Vec<u8>)>) -> Result<()> {
        for (addr, code) in codes {
            let key = keys::code_key(&self.prefix, addr);
            self.conn
                .vkv_set(key, height, serde_json::to_string(&code)?)
                .map_err(|e| Error::RedisError(e))?;
        }
        Ok(())
    }

    pub fn set_account_storages(
        &mut self,
        height: u32,
        account_storages: Vec<((H160, H256), H256)>,
    ) -> Result<()> {
        for ((addr, index), value) in account_storages {
            let key = keys::account_storage_key(&self.prefix, addr, index);
            self.conn
                .vkv_set(key, height, serde_json::to_string(&value)?)
                .map_err(|e| Error::RedisError(e))?;
        }
        Ok(())
    }

    pub fn set_block_info(
        &mut self,
        block: Block,
        receipts: Vec<FrontierReceiptData>,
        statuses: Vec<TransactionStatus>,
        transaction_index: Vec<(H256, (U256, u32))>,
    ) -> Result<()> {
        let block_hash = block.header.hash();
        let height = block.header.number;
        let block_key = keys::block_key(&self.prefix, block_hash);
        self.conn
            .set(block_key, serde_json::to_string(&block)?)
            .map_err(|e| Error::RedisError(e))?;

        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        self.conn
            .set(block_hash_key, serde_json::to_string(&block_hash)?)
            .map_err(|e| Error::RedisError(e))?;

        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        self.conn
            .set(block_height_key, serde_json::to_string(&height)?)
            .map_err(|e| Error::RedisError(e))?;

        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        self.conn
            .vkv_set(
                receipt_key,
                height.as_u32(),
                serde_json::to_string(&receipts)?,
            )
            .map_err(|e| Error::RedisError(e))?;

        let status_key = keys::status_key(&self.prefix, block_hash);
        self.conn
            .vkv_set(
                status_key,
                height.as_u32(),
                serde_json::to_string(&statuses)?,
            )
            .map_err(|e| Error::RedisError(e))?;

        for (tx_hash, hash_index) in transaction_index {
            let transaction_index_key = keys::transaction_index_key(&self.prefix, tx_hash);

            self.conn
                .vkv_set(
                    transaction_index_key,
                    height.as_u32(),
                    serde_json::to_string(&hash_index)?,
                )
                .map_err(|e| Error::RedisError(e))?;
        }
        Ok(())
    }
}
