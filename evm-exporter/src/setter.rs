use {
    crate::{
        error::{Error, Result},
        keys,
        types::{Block, TransactionStatus},
        utils::recover_signer,
        ConnectionType, Receipt,
    },
    ethereum::LegacyTransaction,
    primitive_types::{H160, H256, U256},
    redis::{Commands, Connection},
    redis_versioned_kv::VersionedKVCommand,
    sqlx::PgConnection,
};

pub trait Setter {
    fn new(conn: ConnectionType, something: String) -> Self
    where
        Self: std::marker::Sized;
    fn clear(&mut self) -> Result<()>;
    fn set_height(&mut self, height: u32) -> Result<()>;
    fn set_lowest_height(&mut self, height: u32) -> Result<()>;
    fn set_balance(&mut self, height: u32, address: H160, balance: U256) -> Result<()>;
    fn remove_balance(&mut self, height: u32, address: H160) -> Result<()>;
    fn set_nonce(&mut self, height: u32, address: H160, nonce: U256) -> Result<()>;
    fn remove_nonce(&mut self, height: u32, address: H160) -> Result<()>;
    fn set_byte_code(&mut self, height: u32, address: H160, code: Vec<u8>) -> Result<()>;
    fn remove_byte_code(&mut self, height: u32, address: H160) -> Result<()>;
    fn set_state(&mut self, height: u32, address: H160, index: H256, value: H256) -> Result<()>;
    fn remove_state(&mut self, height: u32, address: H160, index: H256) -> Result<()>;
    fn set_block_info(
        &mut self,
        block: Block,
        receipts: Vec<Receipt>,
        statuses: Vec<TransactionStatus>,
    ) -> Result<()>;
    fn remove_block_info(&mut self, height: U256) -> Result<()>;
    fn set_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()>;
    fn set_pending_code(&mut self, address: H160, code: Vec<u8>) -> Result<()>;
    fn set_pending_state(&mut self, address: H160, index: H256, value: H256) -> Result<()>;
    fn remove_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()>;
    fn remove_pending_code(&mut self, address: H160) -> Result<()>;
    fn remove_pending_state(&mut self, address: H160, index: H256) -> Result<()>;
    fn set_total_issuance(&mut self, height: u32, value: U256) -> Result<()>;
    fn set_allowances(
        &mut self,
        height: u32,
        owner: H160,
        spender: H160,
        value: U256,
    ) -> Result<()>;
}

pub struct PgSetter {
    conn: PgConnection,
}

impl Setter for PgSetter {
    fn new(connection: ConnectionType, _something: String) -> Self {
        if let ConnectionType::Postgres(conn) = connection {
            Self { conn }
        } else {
            panic!("Invalid connection type for Postgres")
        }
    }
    fn clear(&mut self) -> Result<()> {
        Ok(())
    }
    fn set_height(&mut self, height: u32) -> Result<()> {
        Ok(())
    }
    fn set_lowest_height(&mut self, height: u32) -> Result<()> {
        Ok(())
    }
    fn set_balance(&mut self, height: u32, address: H160, balance: U256) -> Result<()> {
        Ok(())
    }
    fn remove_balance(&mut self, height: u32, address: H160) -> Result<()> {
        Ok(())
    }
    fn set_nonce(&mut self, height: u32, address: H160, nonce: U256) -> Result<()> {
        Ok(())
    }
    fn remove_nonce(&mut self, height: u32, address: H160) -> Result<()> {
        Ok(())
    }
    fn set_byte_code(&mut self, height: u32, address: H160, code: Vec<u8>) -> Result<()> {
        Ok(())
    }
    fn remove_byte_code(&mut self, height: u32, address: H160) -> Result<()> {
        Ok(())
    }
    fn set_state(&mut self, height: u32, address: H160, index: H256, value: H256) -> Result<()> {
        Ok(())
    }
    fn remove_state(&mut self, height: u32, address: H160, index: H256) -> Result<()> {
        Ok(())
    }
    fn set_block_info(
        &mut self,
        block: Block,
        receipts: Vec<Receipt>,
        statuses: Vec<TransactionStatus>,
    ) -> Result<()> {
        Ok(())
    }
    fn remove_block_info(&mut self, height: U256) -> Result<()> {
        Ok(())
    }
    fn set_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()> {
        Ok(())
    }
    fn set_pending_code(&mut self, address: H160, code: Vec<u8>) -> Result<()> {
        Ok(())
    }
    fn set_pending_state(&mut self, address: H160, index: H256, value: H256) -> Result<()> {
        Ok(())
    }
    fn remove_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()> {
        Ok(())
    }
    fn remove_pending_code(&mut self, address: H160) -> Result<()> {
        Ok(())
    }
    fn remove_pending_state(&mut self, address: H160, index: H256) -> Result<()> {
        Ok(())
    }
    fn set_total_issuance(&mut self, height: u32, value: U256) -> Result<()> {
        Ok(())
    }
    fn set_allowances(
        &mut self,
        height: u32,
        owner: H160,
        spender: H160,
        value: U256,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct RedisSetter {
    conn: Connection,
    pub prefix: String,
}

impl Setter for RedisSetter {
    fn new(connection: ConnectionType, prefix: String) -> Self {
        if let ConnectionType::Redis(conn) = connection {
            Self { conn, prefix }
        } else {
            panic!("Invalid connection type for Redis")
        }
    }

    fn clear(&mut self) -> Result<()> {
        redis::cmd("FLUSHDB").arg("SYNC").query(&mut self.conn)?;
        Ok(())
    }

    fn set_height(&mut self, height: u32) -> Result<()> {
        let height_key = keys::latest_height_key(&self.prefix);
        self.conn.set(height_key, format!("{}", height))?;
        Ok(())
    }
    fn set_lowest_height(&mut self, height: u32) -> Result<()> {
        let height_key = keys::lowest_height_key(&self.prefix);
        self.conn.set(height_key, format!("{}", height))?;
        Ok(())
    }

    fn set_balance(&mut self, height: u32, address: H160, balance: U256) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        self.conn
            .vkv_set(balance_key, height, serde_json::to_string(&balance)?)?;

        Ok(())
    }

    fn remove_balance(&mut self, height: u32, address: H160) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        self.conn.vkv_del(balance_key, height)?;
        Ok(())
    }

    fn set_nonce(&mut self, height: u32, address: H160, nonce: U256) -> Result<()> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        self.conn
            .vkv_set(nonce_key, height, serde_json::to_string(&nonce)?)?;

        Ok(())
    }
    fn remove_nonce(&mut self, height: u32, address: H160) -> Result<()> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        self.conn.vkv_del(nonce_key, height)?;
        Ok(())
    }
    fn set_byte_code(&mut self, height: u32, address: H160, code: Vec<u8>) -> Result<()> {
        let code_key = keys::code_key(&self.prefix, address);
        self.conn.vkv_set(code_key, height, hex::encode(code))?;

        Ok(())
    }
    fn remove_byte_code(&mut self, height: u32, address: H160) -> Result<()> {
        let code_key = keys::code_key(&self.prefix, address);
        self.conn.vkv_del(code_key, height)?;
        Ok(())
    }

    fn set_state(&mut self, height: u32, address: H160, index: H256, value: H256) -> Result<()> {
        let key = keys::state_key(&self.prefix, address, index);
        self.conn
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        self.conn
            .vkv_set(state_addr_key.clone(), height, state_addr_key)?;
        Ok(())
    }

    fn remove_state(&mut self, height: u32, address: H160, index: H256) -> Result<()> {
        let key = keys::state_key(&self.prefix, address, index);
        self.conn.vkv_del(key, height)?;
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        self.conn.vkv_del(state_addr_key, height)?;
        Ok(())
    }

    fn set_block_info(
        &mut self,
        block: Block,
        receipts: Vec<Receipt>,
        statuses: Vec<TransactionStatus>,
    ) -> Result<()> {
        let block_hash = block.header.hash();
        let height = block.header.number;

        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        self.conn
            .set(block_hash_key, serde_json::to_string(&block_hash)?)?;

        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        self.conn
            .set(block_height_key, serde_json::to_string(&height)?)?;

        let block_key = keys::block_key(&self.prefix, block_hash);
        self.conn.set(block_key, serde_json::to_string(&block)?)?;

        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        self.conn
            .set(receipt_key, serde_json::to_string(&receipts)?)?;

        let status_key = keys::status_key(&self.prefix, block_hash);
        self.conn
            .set(status_key, serde_json::to_string(&statuses)?)?;

        for (i, tx) in statuses.iter().enumerate() {
            let transaction_index_key =
                keys::transaction_index_key(&self.prefix, tx.transaction_hash);
            self.conn.set(
                transaction_index_key,
                serde_json::to_string(&(block_hash, i as u32))?,
            )?;
        }
        Ok(())
    }

    fn remove_block_info(&mut self, height: U256) -> Result<()> {
        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        let block_hash: H256 = match self
            .conn
            .get::<String, Option<String>>(block_hash_key.clone())?
        {
            Some(v) => serde_json::from_str(&v)?,
            None => {
                return Ok(());
            }
        };
        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        let block_key = keys::block_key(&self.prefix, block_hash);
        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        let status_key = keys::status_key(&self.prefix, block_hash);
        let statuses: Vec<TransactionStatus> = match self
            .conn
            .get::<String, Option<String>>(status_key.clone())?
        {
            Some(v) => serde_json::from_str(&v)?,
            None => {
                return Err(Error::ValueNotFound);
            }
        };
        for tx in statuses.iter() {
            let transaction_index_key =
                keys::transaction_index_key(&self.prefix, tx.transaction_hash);
            self.conn.del(transaction_index_key)?;
        }
        self.conn.del(block_height_key)?;
        self.conn.del(block_key)?;
        self.conn.del(receipt_key)?;
        self.conn.del(status_key)?;
        self.conn.del(block_hash_key)?;
        Ok(())
    }

    fn set_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;

        let height_key = keys::latest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get(height_key)?;
        let height = match height {
            Some(str) => str.parse::<u32>()?,
            _ => 0,
        };
        let balance_key = keys::balance_key(&self.prefix, sign_address);
        let balance: Option<String> = self.conn.vkv_get(balance_key, height)?;
        let balance = if let Some(s) = balance {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };

        let pending_balance_key = keys::pending_balance_key(&self.prefix, sign_address);
        let total_payment = transaction
            .value
            .saturating_add(transaction.gas_price.saturating_mul(transaction.gas_limit));
        self.conn.set(
            pending_balance_key,
            serde_json::to_string(&balance.saturating_sub(total_payment))?,
        )?;

        let pending_nonce_key = keys::pending_nonce_key(&self.prefix, sign_address);
        self.conn.set(
            pending_nonce_key,
            serde_json::to_string(&transaction.nonce)?,
        )?;

        Ok(())
    }

    fn set_pending_code(&mut self, address: H160, code: Vec<u8>) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn
            .set(pending_code_key, serde_json::to_string(&code)?)?;
        Ok(())
    }

    fn set_pending_state(&mut self, address: H160, index: H256, value: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn
            .set(pending_state_key, serde_json::to_string(&value)?)?;

        Ok(())
    }

    fn remove_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;
        let pending_balance_key = keys::pending_balance_key(&self.prefix, sign_address);

        self.conn.del(pending_balance_key)?;

        let pending_nonce_key = keys::pending_nonce_key(&self.prefix, sign_address);
        self.conn.del(pending_nonce_key)?;

        Ok(())
    }

    fn remove_pending_code(&mut self, address: H160) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn.del(pending_code_key)?;
        Ok(())
    }

    fn remove_pending_state(&mut self, address: H160, index: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn.del(pending_state_key)?;

        Ok(())
    }

    fn set_total_issuance(&mut self, height: u32, value: U256) -> Result<()> {
        let key = keys::total_issuance_key(&self.prefix);
        self.conn
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        Ok(())
    }

    fn set_allowances(
        &mut self,
        height: u32,
        owner: H160,
        spender: H160,
        value: U256,
    ) -> Result<()> {
        let key = keys::allowances_key(&self.prefix, owner, spender);
        self.conn
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        Ok(())
    }
}
