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
    std::str::FromStr,
};

#[cfg(feature = "redis")]
use { 
    redis_versioned_kv::VersionedKVCommand,
    redis::{Commands, Client as RedisClient },
};

#[cfg(feature = "redis-cluster")]
use redis::cluster::ClusterClient as RedisClusterClient;

#[cfg(feature = "postgres")]
use {
    r2d2::Pool,
    r2d2_postgres::{postgres::NoTls, PostgresConnectionManager},
};

pub trait Setter {
    fn new(conn: ConnectionType, something: String) -> Self
    where
        Self: std::marker::Sized;
    fn clear(&self) -> Result<()>;
    fn set_height(&self, height: u32) -> Result<()>;
    fn set_lowest_height(&self, height: u32) -> Result<()>;
    fn set_balance(&self, height: u32, address: H160, balance: U256) -> Result<()>;
    fn remove_balance(&self, height: u32, address: H160) -> Result<()>;
    fn set_nonce(&self, height: u32, address: H160, nonce: U256) -> Result<()>;
    fn remove_nonce(&self, height: u32, address: H160) -> Result<()>;
    fn set_byte_code(&self, height: u32, address: H160, code: Vec<u8>) -> Result<()>;
    fn remove_byte_code(&self, height: u32, address: H160) -> Result<()>;
    fn set_state(&self, height: u32, address: H160, index: H256, value: H256) -> Result<()>;
    fn remove_state(&self, height: u32, address: H160, index: H256) -> Result<()>;
    fn set_block_info(
        &self,
        block: Block,
        receipts: Vec<Receipt>,
        statuses: Vec<TransactionStatus>,
    ) -> Result<()>;
    fn remove_block_info(&self, height: U256) -> Result<()>;
    fn set_pending_tx(&self, transaction: LegacyTransaction) -> Result<()>;
    fn set_pending_code(&self, address: H160, code: Vec<u8>) -> Result<()>;
    fn set_pending_state(&self, address: H160, index: H256, value: H256) -> Result<()>;
    fn remove_pending_tx(&self, transaction: LegacyTransaction) -> Result<()>;
    fn remove_pending_code(&self, address: H160) -> Result<()>;
    fn remove_pending_state(&self, address: H160, index: H256) -> Result<()>;
    fn set_total_issuance(&self, height: u32, value: U256) -> Result<()>;
    fn set_allowances(
        &self,
        height: u32,
        owner: H160,
        spender: H160,
        value: U256,
    ) -> Result<()>;
}

#[cfg(feature = "postgres")]
pub struct PgSetter {
    conn: Pool<PostgresConnectionManager<NoTls>>,
}

#[cfg(feature = "postgres")]
impl Setter for PgSetter {
    fn new(connection: ConnectionType, _something: String) -> Self {
        if let ConnectionType::Postgres(uri) = connection {
            let manager = PostgresConnectionManager::new(
                uri.parse().expect("parse postgres uri failed"),
                NoTls,
            );
            let pool = r2d2::Pool::new(manager).expect("new postgres connection pool failed");
            Self { conn: pool }
        } else {
            panic!("Invalid connection type for Postgres")
        }
    }
    fn clear(&self) -> Result<()> {
        self.conn.get()?.execute(
            r"TRUNCATE
             allowances,
             block_info,
             common,
             nonce,
             pending_state,
             state,
             balance,
             byte_code,
             issuance,
             pending_byte_code,
             pending_transactions,
             transactions",
            &[],
        )?;
        Ok(())
    }
    fn set_height(&self, height: u32) -> Result<()> {
        self.conn.get()?
            .execute("UPDATE common set latest_height = $1", &[&height])?;
        Ok(())
    }
    fn set_lowest_height(&self, height: u32) -> Result<()> {
        self.conn.get()?
            .execute("UPDATE common set lowest_height = $1", &[&height])?;
        Ok(())
    }
    fn set_balance(&self, height: u32, address: H160, balance: U256) -> Result<()> {
        self.conn.get()?.execute(
            "INSERT INTO balance(balance, address, height) VALUES($1, $2, $3)",
            &[&balance.to_string(), &address.to_string(), &height],
        )?;
        Ok(())
    }
    fn remove_balance(&self, height: u32, address: H160) -> Result<()> {
        self.conn.get()?.execute(
            "DELETE FROM balance WHERE height = $1 AND address = $2",
            &[&height, &address.to_string()],
        )?;
        Ok(())
    }
    fn set_nonce(&self, height: u32, address: H160, nonce: U256) -> Result<()> {
        self.conn.get()?.execute(
            "INSERT INTO nonce(nonce, address, height) VALUES($1, $2, $3)",
            &[&nonce.to_string(), &address.to_string(), &height],
        )?;
        Ok(())
    }
    fn remove_nonce(&self, height: u32, address: H160) -> Result<()> {
        self.conn.get()?.execute(
            "DELETE FROM nonce WHERE height = $1 AND address = $2",
            &[&height, &address.to_string()],
        )?;
        Ok(())
    }
    fn set_byte_code(&self, height: u32, address: H160, code: Vec<u8>) -> Result<()> {
        self.conn.get()?.execute(
            "INSERT INTO byte_code(code, address, height) VALUES($1, $2, $3)",
            &[&hex::encode(code), &address.to_string(), &height],
        )?;
        Ok(())
    }
    fn remove_byte_code(&self, height: u32, address: H160) -> Result<()> {
        self.conn.get()?.execute(
            "DELETE FROM byte_code WHERE height = $1 AND address = $2",
            &[&height, &address.to_string()],
        )?;
        Ok(())
    }
    fn set_state(&self, height: u32, address: H160, index: H256, value: H256) -> Result<()> {
        self.conn.get()?.execute(
            "INSERT INTO state(value, idx, address, height) VALUES($1, $2, $3, $4)",
            &[
                &value.to_string(),
                &index.to_string(),
                &address.to_string(),
                &height,
            ],
        )?;
        Ok(())
    }
    fn remove_state(&self, height: u32, address: H160, index: H256) -> Result<()> {
        self.conn.get()?.execute(
            "DELETE FROM state WHERE height = $1 AND address = $2 AND idx = $3",
            &[&height, &address.to_string(), &index.to_string()],
        )?;
        Ok(())
    }
    fn set_block_info(
        &self,
        block: Block,
        receipts: Vec<Receipt>,
        statuses: Vec<TransactionStatus>,
    ) -> Result<()> {
        self.conn.get()?.execute(
            r"INSERT INTO block_info(block_hash, block_height, block, receipt, statuses) 
            VALUES($1, $2, $3, $4, $5)",
            &[
                &block.header.hash().to_string(),
                &block.header.number.to_string(),
                &serde_json::to_string(&block)?,
                &serde_json::to_string(&receipts)?,
                &serde_json::to_string(&statuses)?,
            ],
        )?;

        for (i, tx) in statuses.iter().enumerate() {
            self.conn.get()?.execute(
                "INSERT INTO transactions(transaction_hash, transaction_index) VALUES($1, $2))",
                &[
                    &tx.transaction_hash.to_string(),
                    &serde_json::to_string(&(block.header.hash().to_string(), i as u32))?,
                ],
            )?;
        }
        Ok(())
    }
    fn remove_block_info(&self, block_height: U256) -> Result<()> {
        let row: String = self
            .conn
            .get()?
            .query_one(
                "SELECT statuses FROM block_info WHERE block_height = $1",
                &[&block_height.to_string()],
            )?
            .get("statuses");
        let statuses: Vec<TransactionStatus> = serde_json::from_str(&row)?;

        self.conn.get()?.execute(
            "DELETE FROM block_info WHERE block_height = $1",
            &[&block_height.to_string()],
        )?;

        for tx in statuses {
            self.conn.get()?.execute(
                "DELETE FROM transactions WHERE transaction_hash = $1",
                &[&tx.transaction_hash.to_string()],
            )?;
        }

        Ok(())
    }
    fn set_pending_tx(&self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;
        let latest_height: u32 = self
            .conn
            .get()?
            .query_one("SELECT latest_height FROM common", &[])?
            .get("latest_height");
        let balance = U256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT balance FROM balance WHERE address = $1 AND height = $2",
                    &[&sign_address.to_string(), &latest_height],
                )?
                .get("balance"),
        )?;

        self.conn.get()?.execute(
            "INSERT INTO pending_transactions(sign_address, pending_balance, pending_nonce) VALUES($1, $2, $3)",
            &[
                &sign_address.to_string(),
                &balance.saturating_sub(transaction.value.saturating_add(transaction.gas_price.saturating_mul(transaction.gas_limit))).to_string(),
                &transaction.nonce.to_string(),
            ],
            
        )?;
        Ok(())
    }
    fn set_pending_code(&self, address: H160, code: Vec<u8>) -> Result<()> {
        self.conn.get()?.execute("INSERT INTO pending_byte_code(code, address) VALUES($1, $2)", 
            &[&serde_json::to_string(&code)?, &address.to_string()],
        )?;
        Ok(())
    }
    fn set_pending_state(&self, address: H160, index: H256, value: H256) -> Result<()> {
        self.conn.get()?.execute("INSERT INTO pending_state(value, idx, address) VALUES($1, $2, $3)", 
            &[&value.to_string(), &index.to_string(), &address.to_string()],
        )?;
        Ok(())
    }
    fn remove_pending_tx(&self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;
        self.conn.get()?.execute("DELETE FROM pending_transactions WHERE sign_address = $1", &[&sign_address.to_string()])?;
        Ok(())
    }
    fn remove_pending_code(&self, address: H160) -> Result<()> {
        self.conn.get()?.execute("DELETE FROM pending_byte_code WHERE address = $1", &[&address.to_string()])?;
        Ok(())
    }
    fn remove_pending_state(&self, address: H160, index: H256) -> Result<()> {
        self.conn.get()?.execute("DELETE FROM pending_state WHERE address = $1 AND idx = $2", &[&address.to_string(), &index.to_string()])?;
        Ok(())
    }
    fn set_total_issuance(&self, height: u32, value: U256) -> Result<()> {
        self.conn.get()?.execute("INSERT INTO issuance(value, height) VALUES($1, $2)", &[&value.to_string(), &height])?;
        Ok(())
    }
    fn set_allowances(
        &self,
        height: u32,
        owner: H160,
        spender: H160,
        value: U256,
    ) -> Result<()> {
        self.conn.get()?.execute("INSERT INTO allowances(owner, spender, value, height) VALUES($1, $2, $3, $4)", 
            &[&owner.to_string(), &spender.to_string(), &value.to_string(),&height],
        )?;
        Ok(())
    }
}

#[cfg(feature = "redis")]
pub struct RedisSetter {
    conn: RedisClient,
    pub prefix: String,
}

#[cfg(feature = "redis")]
impl Setter for RedisSetter {
    fn new(connection: ConnectionType, prefix: String) -> Self {
        if let ConnectionType::Redis(url) = connection {
            Self {
                conn: RedisClient::open(url).expect("Connect to Redis failed"),
                prefix,
            }
        } else {
            panic!("Invalid connection type for Redis")
        }
    }

    fn clear(&self) -> Result<()> {
        redis::cmd("FLUSHDB").arg("SYNC").query(&mut self.conn.get_connection()?)?;
        Ok(())
    }

    fn set_height(&self, height: u32) -> Result<()> {
        let height_key = keys::latest_height_key(&self.prefix);
        self.conn.get_connection()?.set(height_key, format!("{}", height))?;
        Ok(())
    }
    fn set_lowest_height(&self, height: u32) -> Result<()> {
        let height_key = keys::lowest_height_key(&self.prefix);
        self.conn.get_connection()?.set(height_key, format!("{}", height))?;
        Ok(())
    }

    fn set_balance(&self, height: u32, address: H160, balance: U256) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        self.conn.get_connection()?
            .vkv_set(balance_key, height, serde_json::to_string(&balance)?)?;

        Ok(())
    }

    fn remove_balance(&self, height: u32, address: H160) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(balance_key, height)?;
        Ok(())
    }

    fn set_nonce(&self, height: u32, address: H160, nonce: U256) -> Result<()> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        self.conn.get_connection()?
            .vkv_set(nonce_key, height, serde_json::to_string(&nonce)?)?;

        Ok(())
    }
    fn remove_nonce(&self, height: u32, address: H160) -> Result<()> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(nonce_key, height)?;
        Ok(())
    }
    fn set_byte_code(&self, height: u32, address: H160, code: Vec<u8>) -> Result<()> {
        let code_key = keys::code_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_set(code_key, height, hex::encode(code))?;

        Ok(())
    }
    fn remove_byte_code(&self, height: u32, address: H160) -> Result<()> {
        let code_key = keys::code_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(code_key, height)?;
        Ok(())
    }

    fn set_state(&self, height: u32, address: H160, index: H256, value: H256) -> Result<()> {
        let key = keys::state_key(&self.prefix, address, index);
        self.conn.get_connection()?
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        self.conn.get_connection()?
            .vkv_set(state_addr_key.clone(), height, state_addr_key)?;
        Ok(())
    }

    fn remove_state(&self, height: u32, address: H160, index: H256) -> Result<()> {
        let key = keys::state_key(&self.prefix, address, index);
        self.conn.get_connection()?.vkv_del(key, height)?;
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(state_addr_key, height)?;
        Ok(())
    }

    fn set_block_info(
        &self,
        block: Block,
        receipts: Vec<Receipt>,
        statuses: Vec<TransactionStatus>,
    ) -> Result<()> {
        let block_hash = block.header.hash();
        let height = block.header.number;

        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        self.conn.get_connection()?
            .set(block_hash_key, serde_json::to_string(&block_hash)?)?;

        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        self.conn.get_connection()?
            .set(block_height_key, serde_json::to_string(&height)?)?;

        let block_key = keys::block_key(&self.prefix, block_hash);
        self.conn.get_connection()?.set(block_key, serde_json::to_string(&block)?)?;

        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        self.conn.get_connection()?
            .set(receipt_key, serde_json::to_string(&receipts)?)?;

        let status_key = keys::status_key(&self.prefix, block_hash);
        self.conn.get_connection()?
            .set(status_key, serde_json::to_string(&statuses)?)?;

        for (i, tx) in statuses.iter().enumerate() {
            let transaction_index_key =
                keys::transaction_index_key(&self.prefix, tx.transaction_hash);
            self.conn.get_connection()?.set(
                transaction_index_key,
                serde_json::to_string(&(block_hash, i as u32))?,
            )?;
        }
        Ok(())
    }

    fn remove_block_info(&self, height: U256) -> Result<()> {
        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        let block_hash: H256 = match self.conn.get_connection()?
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
            .conn.get_connection()?
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
            self.conn.get_connection()?.del(transaction_index_key)?;
        }
        self.conn.get_connection()?.del(block_height_key)?;
        self.conn.get_connection()?.del(block_key)?;
        self.conn.get_connection()?.del(receipt_key)?;
        self.conn.get_connection()?.del(status_key)?;
        self.conn.get_connection()?.del(block_hash_key)?;
        Ok(())
    }

    fn set_pending_tx(&self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;

        let height_key = keys::latest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get_connection()?.get(height_key)?;
        let height = match height {
            Some(str) => str.parse::<u32>()?,
            _ => 0,
        };
        let balance_key = keys::balance_key(&self.prefix, sign_address);
        let balance: Option<String> = self.conn.get_connection()?.vkv_get(balance_key, height)?;
        let balance = if let Some(s) = balance {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };

        let pending_balance_key = keys::pending_balance_key(&self.prefix, sign_address);
        let total_payment = transaction
            .value
            .saturating_add(transaction.gas_price.saturating_mul(transaction.gas_limit));
        self.conn.get_connection()?.set(
            pending_balance_key,
            serde_json::to_string(&balance.saturating_sub(total_payment))?,
        )?;

        let pending_nonce_key = keys::pending_nonce_key(&self.prefix, sign_address);
        self.conn.get_connection()?.set(
            pending_nonce_key,
            serde_json::to_string(&transaction.nonce)?,
        )?;

        Ok(())
    }

    fn set_pending_code(&self, address: H160, code: Vec<u8>) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn.get_connection()?
            .set(pending_code_key, serde_json::to_string(&code)?)?;
        Ok(())
    }

    fn set_pending_state(&self, address: H160, index: H256, value: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn.get_connection()?
            .set(pending_state_key, serde_json::to_string(&value)?)?;

        Ok(())
    }

    fn remove_pending_tx(&self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;
        let pending_balance_key = keys::pending_balance_key(&self.prefix, sign_address);

        self.conn.get_connection()?.del(pending_balance_key)?;

        let pending_nonce_key = keys::pending_nonce_key(&self.prefix, sign_address);
        self.conn.get_connection()?.del(pending_nonce_key)?;

        Ok(())
    }

    fn remove_pending_code(&self, address: H160) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn.get_connection()?.del(pending_code_key)?;
        Ok(())
    }

    fn remove_pending_state(&self, address: H160, index: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn.get_connection()?.del(pending_state_key)?;

        Ok(())
    }

    fn set_total_issuance(&self, height: u32, value: U256) -> Result<()> {
        let key = keys::total_issuance_key(&self.prefix);
        self.conn.get_connection()?
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        Ok(())
    }

    fn set_allowances(
        &self,
        height: u32,
        owner: H160,
        spender: H160,
        value: U256,
    ) -> Result<()> {
        let key = keys::allowances_key(&self.prefix, owner, spender);
        self.conn.get_connection()?
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        Ok(())
    }
}

#[cfg(feature = "redis-cluster")]
pub struct RedisClusterSetter {
    conn: RedisClusterClient,
    pub prefix: String,
}

#[cfg(feature = "redis-cluster")]
impl Setter for RedisClusterSetter {
    fn new(connection: ConnectionType, prefix: String) -> Self {
        if let ConnectionType::RedisCluster(urls) = connection {
            Self {
                conn: RedisClusterClient::new(urls.to_vec())
                    .expect("Connect to Redis Cluster failed"),
                prefix,
            }
        } else {
            panic!("Invalid connection type for Redis Cluster")
        }
    }

    fn clear(&self) -> Result<()> {
        redis::cmd("FLUSHDB").arg("SYNC").query(&mut self.conn.get_connection()?)?;
        Ok(())
    }

    fn set_height(&self, height: u32) -> Result<()> {
        let height_key = keys::latest_height_key(&self.prefix);
        self.conn.get_connection()?.set(height_key, format!("{}", height))?;
        Ok(())
    }
    fn set_lowest_height(&self, height: u32) -> Result<()> {
        let height_key = keys::lowest_height_key(&self.prefix);
        self.conn.get_connection()?.set(height_key, format!("{}", height))?;
        Ok(())
    }

    fn set_balance(&self, height: u32, address: H160, balance: U256) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        self.conn.get_connection()?
            .vkv_set(balance_key, height, serde_json::to_string(&balance)?)?;

        Ok(())
    }

    fn remove_balance(&self, height: u32, address: H160) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(balance_key, height)?;
        Ok(())
    }

    fn set_nonce(&self, height: u32, address: H160, nonce: U256) -> Result<()> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        self.conn.get_connection()?
            .vkv_set(nonce_key, height, serde_json::to_string(&nonce)?)?;

        Ok(())
    }
    fn remove_nonce(&self, height: u32, address: H160) -> Result<()> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(nonce_key, height)?;
        Ok(())
    }
    fn set_byte_code(&self, height: u32, address: H160, code: Vec<u8>) -> Result<()> {
        let code_key = keys::code_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_set(code_key, height, hex::encode(code))?;

        Ok(())
    }
    fn remove_byte_code(&self, height: u32, address: H160) -> Result<()> {
        let code_key = keys::code_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(code_key, height)?;
        Ok(())
    }

    fn set_state(&self, height: u32, address: H160, index: H256, value: H256) -> Result<()> {
        let key = keys::state_key(&self.prefix, address, index);
        self.conn.get_connection()?
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        self.conn.get_connection()?
            .vkv_set(state_addr_key.clone(), height, state_addr_key)?;
        Ok(())
    }

    fn remove_state(&self, height: u32, address: H160, index: H256) -> Result<()> {
        let key = keys::state_key(&self.prefix, address, index);
        self.conn.get_connection()?.vkv_del(key, height)?;
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        self.conn.get_connection()?.vkv_del(state_addr_key, height)?;
        Ok(())
    }

    fn set_block_info(
        &self,
        block: Block,
        receipts: Vec<Receipt>,
        statuses: Vec<TransactionStatus>,
    ) -> Result<()> {
        let block_hash = block.header.hash();
        let height = block.header.number;

        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        self.conn.get_connection()?
            .set(block_hash_key, serde_json::to_string(&block_hash)?)?;

        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        self.conn.get_connection()?
            .set(block_height_key, serde_json::to_string(&height)?)?;

        let block_key = keys::block_key(&self.prefix, block_hash);
        self.conn.get_connection()?.set(block_key, serde_json::to_string(&block)?)?;

        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        self.conn.get_connection()?
            .set(receipt_key, serde_json::to_string(&receipts)?)?;

        let status_key = keys::status_key(&self.prefix, block_hash);
        self.conn.get_connection()?
            .set(status_key, serde_json::to_string(&statuses)?)?;

        for (i, tx) in statuses.iter().enumerate() {
            let transaction_index_key =
                keys::transaction_index_key(&self.prefix, tx.transaction_hash);
            self.conn.get_connection()?.set(
                transaction_index_key,
                serde_json::to_string(&(block_hash, i as u32))?,
            )?;
        }
        Ok(())
    }

    fn remove_block_info(&self, height: U256) -> Result<()> {
        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        let block_hash: H256 = match self.conn.get_connection()?
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
            .conn.get_connection()?
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
            self.conn.get_connection()?.del(transaction_index_key)?;
        }
        self.conn.get_connection()?.del(block_height_key)?;
        self.conn.get_connection()?.del(block_key)?;
        self.conn.get_connection()?.del(receipt_key)?;
        self.conn.get_connection()?.del(status_key)?;
        self.conn.get_connection()?.del(block_hash_key)?;
        Ok(())
    }

    fn set_pending_tx(&self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;

        let height_key = keys::latest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get_connection()?.get(height_key)?;
        let height = match height {
            Some(str) => str.parse::<u32>()?,
            _ => 0,
        };
        let balance_key = keys::balance_key(&self.prefix, sign_address);
        let balance: Option<String> = self.conn.get_connection()?.vkv_get(balance_key, height)?;
        let balance = if let Some(s) = balance {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };

        let pending_balance_key = keys::pending_balance_key(&self.prefix, sign_address);
        let total_payment = transaction
            .value
            .saturating_add(transaction.gas_price.saturating_mul(transaction.gas_limit));
        self.conn.get_connection()?.set(
            pending_balance_key,
            serde_json::to_string(&balance.saturating_sub(total_payment))?,
        )?;

        let pending_nonce_key = keys::pending_nonce_key(&self.prefix, sign_address);
        self.conn.get_connection()?.set(
            pending_nonce_key,
            serde_json::to_string(&transaction.nonce)?,
        )?;

        Ok(())
    }

    fn set_pending_code(&self, address: H160, code: Vec<u8>) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn.get_connection()?
            .set(pending_code_key, serde_json::to_string(&code)?)?;
        Ok(())
    }

    fn set_pending_state(&self, address: H160, index: H256, value: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn.get_connection()?
            .set(pending_state_key, serde_json::to_string(&value)?)?;

        Ok(())
    }

    fn remove_pending_tx(&self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;
        let pending_balance_key = keys::pending_balance_key(&self.prefix, sign_address);

        self.conn.get_connection()?.del(pending_balance_key)?;

        let pending_nonce_key = keys::pending_nonce_key(&self.prefix, sign_address);
        self.conn.get_connection()?.del(pending_nonce_key)?;

        Ok(())
    }

    fn remove_pending_code(&self, address: H160) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn.get_connection()?.del(pending_code_key)?;
        Ok(())
    }

    fn remove_pending_state(&self, address: H160, index: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn.get_connection()?.del(pending_state_key)?;

        Ok(())
    }

    fn set_total_issuance(&self, height: u32, value: U256) -> Result<()> {
        let key = keys::total_issuance_key(&self.prefix);
        self.conn.get_connection()?
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        Ok(())
    }

    fn set_allowances(
        &self,
        height: u32,
        owner: H160,
        spender: H160,
        value: U256,
    ) -> Result<()> {
        let key = keys::allowances_key(&self.prefix, owner, spender);
        self.conn.get_connection()?
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        Ok(())
    }
}
