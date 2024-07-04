use {
    crate::{keys, AccountBasic, Block, ConnectionType, Receipt, Result, TransactionStatus},
    primitive_types::{H160, H256, U256},
    std::str::FromStr,
};

#[cfg(feature = "redis")]
use {
    redis::{Client as RedisClient, Commands},
    redis_versioned_kv::VersionedKVCommand,
};

#[cfg(feature = "redis-cluster")]
use redis::cluster::ClusterClient as RedisClusterClient;

#[cfg(feature = "postgres")]
use {
    r2d2::Pool,
    r2d2_postgres::{postgres::NoTls, PostgresConnectionManager},
};

pub trait Getter {
    fn new(conn: ConnectionType, something: String) -> Self
    where
        Self: std::marker::Sized;
    fn latest_height(&self) -> Result<u32>;
    fn lowest_height(&self) -> Result<u32>;
    fn get_balance(&self, height: u32, address: H160) -> Result<U256>;
    fn get_nonce(&self, height: u32, address: H160) -> Result<U256>;
    fn get_byte_code(&self, height: u32, address: H160) -> Result<Vec<u8>>;
    fn get_account_basic(&self, height: u32, address: H160) -> Result<AccountBasic>;
    fn addr_state_exists(&self, height: u32, address: H160) -> Result<bool>;
    fn get_state(&self, height: u32, address: H160, index: H256) -> Result<H256>;
    fn get_block_hash_by_height(&self, height: U256) -> Result<Option<H256>>;
    fn get_height_by_block_hash(&self, block_hash: H256) -> Result<Option<U256>>;
    fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<Block>>;
    fn get_transaction_receipt_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<Receipt>>>;
    fn get_transaction_status_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<TransactionStatus>>>;
    fn get_transaction_index_by_tx_hash(&self, tx_hash: H256) -> Result<Option<(H256, u32)>>;
    fn get_pending_balance(&self, address: H160) -> Result<Option<U256>>;
    fn get_pending_nonce(&self, address: H160) -> Result<Option<U256>>;
    fn get_pending_byte_code(&self, address: H160) -> Result<Option<Vec<u8>>>;
    fn get_pending_state(&self, address: H160, index: H256) -> Result<Option<H256>>;
    fn get_total_issuance(&self, height: u32) -> Result<U256>;
    fn get_allowances(&self, height: u32, owner: H160, spender: H160) -> Result<U256>;
}

#[cfg(feature = "postgres")]
pub struct PgGetter {
    conn: Pool<PostgresConnectionManager<NoTls>>,
}

#[cfg(feature = "postgres")]
impl Getter for PgGetter {
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
    fn latest_height(&self) -> Result<u32> {
        Ok(self
            .conn
            .get()?
            .query_one("SELECT latest_height FROM common", &[])?
            .get("latest_height"))
    }
    fn lowest_height(&self) -> Result<u32> {
        Ok(self
            .conn
            .get()?
            .query_one("SELECT lowest_height FROM common", &[])?
            .get("lowest_height"))
    }
    fn get_balance(&self, height: u32, address: H160) -> Result<U256> {
        Ok(U256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT balance FROM balance WHERE address = $1 AND height = $2",
                    &[&address.to_string(), &height],
                )?
                .get("balance"),
        )?)
    }
    fn get_nonce(&self, height: u32, address: H160) -> Result<U256> {
        Ok(U256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT nonce FROM nonce WHERE address = $1 AND height = $2",
                    &[&address.to_string(), &height],
                )?
                .get("nonce"),
        )?)
    }
    fn get_byte_code(&self, height: u32, address: H160) -> Result<Vec<u8>> {
        Ok(hex::decode::<Vec<u8>>(
            self.conn
                .get()?
                .query_one(
                    "SELECT code FROM byte_code WHERE address = $1 AND height = $2",
                    &[&address.to_string(), &height],
                )?
                .get("code"),
        )?)
    }
    fn get_account_basic(&self, height: u32, address: H160) -> Result<AccountBasic> {
        Ok(AccountBasic {
            balance: self.get_balance(height, address)?,
            code: self.get_byte_code(height, address)?,
            nonce: self.get_nonce(height, address)?,
        })
    }
    fn addr_state_exists(&self, height: u32, address: H160) -> Result<bool> {
        Ok(!self
            .conn
            .get()?
            .query_one(
                "SELECT 1 FROM state WHERE address = $1 AND height = $2",
                &[&address.to_string(), &height],
            )?
            .is_empty())
    }
    fn get_state(&self, height: u32, address: H160, index: H256) -> Result<H256> {
        Ok(H256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT value FROM state WHERE idx = $1 AND address = $2 AND height = $3",
                    &[&index.to_string(), &address.to_string(), &height],
                )?
                .get("value"),
        )?)
    }
    fn get_block_hash_by_height(&self, height: U256) -> Result<Option<H256>> {
        Ok(Some(H256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT block_hash FROM block_info WHERE block_height = $1",
                    &[&height.to_string()],
                )?
                .get("block_hash"),
        )?))
    }
    fn get_height_by_block_hash(&self, block_hash: H256) -> Result<Option<U256>> {
        Ok(Some(U256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT block_height FROM block_info WHERE block_hash = $1",
                    &[&block_hash.to_string()],
                )?
                .get("block_height"),
        )?))
    }
    fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<Block>> {
        Ok(Some(serde_json::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT block FROM block_info WHERE block_hash = $1",
                    &[&block_hash.to_string()],
                )?
                .get("block"),
        )?))
    }
    fn get_transaction_receipt_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<Receipt>>> {
        Ok(Some(serde_json::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT receipt FROM block_info WHERE block_hash = $1",
                    &[&block_hash.to_string()],
                )?
                .get("receipt"),
        )?))
    }
    fn get_transaction_status_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<TransactionStatus>>> {
        Ok(Some(serde_json::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT statuses FROM block_info WHERE block_hash = $1",
                    &[&block_hash.to_string()],
                )?
                .get("statuses"),
        )?))
    }
    fn get_transaction_index_by_tx_hash(&self, tx_hash: H256) -> Result<Option<(H256, u32)>> {
        Ok(Some(serde_json::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT transaction_index FROM transactions WHERE transaction_hash = $1",
                    &[&tx_hash.to_string()],
                )?
                .get("transaction_index"),
        )?))
    }
    fn get_pending_balance(&self, address: H160) -> Result<Option<U256>> {
        Ok(Some(U256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT pending_balance FROM pending_transactions WHERE sign_address = $1",
                    &[&address.to_string()],
                )?
                .get("pending_balance"),
        )?))
    }
    fn get_pending_nonce(&self, address: H160) -> Result<Option<U256>> {
        Ok(Some(U256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT pending_nonce FROM pending_transactions WHERE sign_address = $1",
                    &[&address.to_string()],
                )?
                .get("pending_nonce"),
        )?))
    }
    fn get_pending_byte_code(&self, address: H160) -> Result<Option<Vec<u8>>> {
        Ok(Some(serde_json::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT code FROM pending_byte_code WHERE address = $1",
                    &[&address.to_string()],
                )?
                .get("code"),
        )?))
    }
    fn get_pending_state(&self, address: H160, index: H256) -> Result<Option<H256>> {
        Ok(Some(H256::from_str(
            self.conn
                .get()?
                .query_one(
                    "SELECT value FROM pending_state WHERE address = $1 AND idx = $2",
                    &[&address.to_string(), &index.to_string()],
                )?
                .get("value"),
        )?))
    }
    fn get_total_issuance(&self, height: u32) -> Result<U256> {
        Ok(U256::from_str(
            self.conn
                .get()?
                .query_one("SELECT value FROM issuance WHERE height = $1", &[&height])?
                .get("value"),
        )?)
    }
    fn get_allowances(&self, height: u32, owner: H160, spender: H160) -> Result<U256> {
        Ok(U256::from_str(
            self.conn
                .get()?
                .query_one("SELECT value FROM allowances WHERE owner = $1 AND spender = $2 AND height = $3", &[&owner.to_string(),&spender.to_string(),&height])?
                .get("value"),
        )?)
    }
}

#[cfg(feature = "redis")]
pub struct RedisGetter {
    conn: RedisClient,
    pub prefix: String,
}

#[cfg(feature = "redis")]
impl Getter for RedisGetter {
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

    fn latest_height(&self) -> Result<u32> {
        let height_key = keys::latest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get_connection()?.get(height_key)?;
        match height {
            Some(str) => Ok(str.parse::<u32>()?),
            _ => Ok(0),
        }
    }
    fn lowest_height(&self) -> Result<u32> {
        let height_key = keys::lowest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get_connection()?.get(height_key)?;
        match height {
            Some(str) => Ok(str.parse::<u32>()?),
            _ => Ok(0),
        }
    }
    fn get_balance(&self, height: u32, address: H160) -> Result<U256> {
        let balance_key = keys::balance_key(&self.prefix, address);
        let balance: Option<String> = self.conn.get_connection()?.vkv_get(balance_key, height)?;
        let balance = if let Some(s) = balance {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(balance)
    }

    fn get_nonce(&self, height: u32, address: H160) -> Result<U256> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        let nonce: Option<String> = self.conn.get_connection()?.vkv_get(nonce_key, height)?;
        let nonce = if let Some(s) = nonce {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(nonce)
    }

    fn get_byte_code(&self, height: u32, address: H160) -> Result<Vec<u8>> {
        let code_key = keys::code_key(&self.prefix, address);
        let code: Option<String> = self.conn.get_connection()?.vkv_get(code_key, height)?;
        let code = if let Some(s) = code {
            hex::decode(s)?
        } else {
            Vec::new()
        };
        Ok(code)
    }

    fn get_account_basic(&self, height: u32, address: H160) -> Result<AccountBasic> {
        Ok(AccountBasic {
            balance: self.get_balance(height, address)?,
            code: self.get_byte_code(height, address)?,
            nonce: self.get_nonce(height, address)?,
        })
    }

    fn addr_state_exists(&self, height: u32, address: H160) -> Result<bool> {
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .vkv_get(state_addr_key, height)?;
        Ok(value.is_some())
    }

    fn get_state(&self, height: u32, address: H160, index: H256) -> Result<H256> {
        let state_key = keys::state_key(&self.prefix, address, index);
        let value: Option<String> = self.conn.get_connection()?.vkv_get(state_key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            H256::zero()
        };
        Ok(val)
    }

    fn get_block_hash_by_height(&self, height: U256) -> Result<Option<H256>> {
        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .get::<&str, Option<String>>(&block_hash_key)?;
        if let Some(hash) = value {
            Ok(Some(serde_json::from_str(hash.as_str())?))
        } else {
            Ok(None)
        }
    }

    fn get_height_by_block_hash(&self, block_hash: H256) -> Result<Option<U256>> {
        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .get::<&str, Option<String>>(&block_height_key)?;
        if let Some(hash) = value {
            Ok(Some(serde_json::from_str(hash.as_str())?))
        } else {
            Ok(None)
        }
    }

    fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<Block>> {
        let block_key = keys::block_key(&self.prefix, block_hash);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .get::<&str, Option<String>>(&block_key)?;
        if let Some(block) = value {
            Ok(Some(serde_json::from_str(block.as_str())?))
        } else {
            Ok(None)
        }
    }

    fn get_transaction_receipt_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<Receipt>>> {
        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        let value: Option<String> = self.conn.get_connection()?.get(receipt_key)?;

        match value {
            Some(receipts) => Ok(serde_json::from_str(receipts.as_str())?),
            _ => Ok(None),
        }
    }

    fn get_transaction_status_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<TransactionStatus>>> {
        let status_key = keys::status_key(&self.prefix, block_hash);

        let value: Option<String> = self.conn.get_connection()?.get(status_key)?;

        match value {
            Some(statuses) => Ok(serde_json::from_str(statuses.as_str())?),
            _ => Ok(None),
        }
    }

    fn get_transaction_index_by_tx_hash(&self, tx_hash: H256) -> Result<Option<(H256, u32)>> {
        let transaction_index_key = keys::transaction_index_key(&self.prefix, tx_hash);

        let value: Option<String> = self.conn.get_connection()?.get(transaction_index_key)?;

        match value {
            Some(hash_index) => Ok(serde_json::from_str(hash_index.as_str())?),
            _ => Ok(None),
        }
    }

    fn get_pending_balance(&self, address: H160) -> Result<Option<U256>> {
        let balance_key = keys::pending_balance_key(&self.prefix, address);
        let balance: Option<String> = self.conn.get_connection()?.get(balance_key)?;
        let balance = if let Some(s) = balance {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(balance)
    }

    fn get_pending_nonce(&self, address: H160) -> Result<Option<U256>> {
        let nonce_key = keys::pending_nonce_key(&self.prefix, address);
        let nonce: Option<String> = self.conn.get_connection()?.get(nonce_key)?;
        let nonce = if let Some(s) = nonce {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(nonce)
    }

    fn get_pending_byte_code(&self, address: H160) -> Result<Option<Vec<u8>>> {
        let code_key = keys::pending_code_key(&self.prefix, address);
        let code: Option<String> = self.conn.get_connection()?.get(code_key)?;
        let code = if let Some(s) = code {
            Some(hex::decode(s)?)
        } else {
            None
        };
        Ok(code)
    }

    fn get_pending_state(&self, address: H160, index: H256) -> Result<Option<H256>> {
        let state_key = keys::pending_state_key(&self.prefix, address, index);
        let value: Option<String> = self.conn.get_connection()?.get(state_key)?;
        let val = if let Some(s) = value {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(val)
    }

    fn get_total_issuance(&self, height: u32) -> Result<U256> {
        let key = keys::total_issuance_key(&self.prefix);
        let value: Option<String> = self.conn.get_connection()?.vkv_get(key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(val)
    }

    fn get_allowances(&self, height: u32, owner: H160, spender: H160) -> Result<U256> {
        let key = keys::allowances_key(&self.prefix, owner, spender);
        let value: Option<String> = self.conn.get_connection()?.vkv_get(key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(val)
    }
}

#[cfg(feature = "redis-cluster")]
pub struct RedisClusterGetter {
    conn: RedisClusterClient,
    pub prefix: String,
}

#[cfg(feature = "redis-cluster")]
impl Getter for RedisClusterGetter {
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

    fn latest_height(&self) -> Result<u32> {
        let height_key = keys::latest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get_connection()?.get(height_key)?;
        match height {
            Some(str) => Ok(str.parse::<u32>()?),
            _ => Ok(0),
        }
    }
    fn lowest_height(&self) -> Result<u32> {
        let height_key = keys::lowest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get_connection()?.get(height_key)?;
        match height {
            Some(str) => Ok(str.parse::<u32>()?),
            _ => Ok(0),
        }
    }
    fn get_balance(&self, height: u32, address: H160) -> Result<U256> {
        let balance_key = keys::balance_key(&self.prefix, address);
        let balance: Option<String> = self.conn.get_connection()?.vkv_get(balance_key, height)?;
        let balance = if let Some(s) = balance {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(balance)
    }

    fn get_nonce(&self, height: u32, address: H160) -> Result<U256> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        let nonce: Option<String> = self.conn.get_connection()?.vkv_get(nonce_key, height)?;
        let nonce = if let Some(s) = nonce {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(nonce)
    }

    fn get_byte_code(&self, height: u32, address: H160) -> Result<Vec<u8>> {
        let code_key = keys::code_key(&self.prefix, address);
        let code: Option<String> = self.conn.get_connection()?.vkv_get(code_key, height)?;
        let code = if let Some(s) = code {
            hex::decode(s)?
        } else {
            Vec::new()
        };
        Ok(code)
    }

    fn get_account_basic(&self, height: u32, address: H160) -> Result<AccountBasic> {
        Ok(AccountBasic {
            balance: self.get_balance(height, address)?,
            code: self.get_byte_code(height, address)?,
            nonce: self.get_nonce(height, address)?,
        })
    }

    fn addr_state_exists(&self, height: u32, address: H160) -> Result<bool> {
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .vkv_get(state_addr_key, height)?;
        Ok(value.is_some())
    }

    fn get_state(&self, height: u32, address: H160, index: H256) -> Result<H256> {
        let state_key = keys::state_key(&self.prefix, address, index);
        let value: Option<String> = self.conn.get_connection()?.vkv_get(state_key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            H256::zero()
        };
        Ok(val)
    }

    fn get_block_hash_by_height(&self, height: U256) -> Result<Option<H256>> {
        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .get::<&str, Option<String>>(&block_hash_key)?;
        if let Some(hash) = value {
            Ok(Some(serde_json::from_str(hash.as_str())?))
        } else {
            Ok(None)
        }
    }

    fn get_height_by_block_hash(&self, block_hash: H256) -> Result<Option<U256>> {
        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .get::<&str, Option<String>>(&block_height_key)?;
        if let Some(hash) = value {
            Ok(Some(serde_json::from_str(hash.as_str())?))
        } else {
            Ok(None)
        }
    }

    fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<Block>> {
        let block_key = keys::block_key(&self.prefix, block_hash);
        let value: Option<String> = self
            .conn
            .get_connection()?
            .get::<&str, Option<String>>(&block_key)?;
        if let Some(block) = value {
            Ok(Some(serde_json::from_str(block.as_str())?))
        } else {
            Ok(None)
        }
    }

    fn get_transaction_receipt_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<Receipt>>> {
        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        let value: Option<String> = self.conn.get_connection()?.get(receipt_key)?;

        match value {
            Some(receipts) => Ok(serde_json::from_str(receipts.as_str())?),
            _ => Ok(None),
        }
    }

    fn get_transaction_status_by_block_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<Vec<TransactionStatus>>> {
        let status_key = keys::status_key(&self.prefix, block_hash);

        let value: Option<String> = self.conn.get_connection()?.get(status_key)?;

        match value {
            Some(statuses) => Ok(serde_json::from_str(statuses.as_str())?),
            _ => Ok(None),
        }
    }

    fn get_transaction_index_by_tx_hash(&self, tx_hash: H256) -> Result<Option<(H256, u32)>> {
        let transaction_index_key = keys::transaction_index_key(&self.prefix, tx_hash);

        let value: Option<String> = self.conn.get_connection()?.get(transaction_index_key)?;

        match value {
            Some(hash_index) => Ok(serde_json::from_str(hash_index.as_str())?),
            _ => Ok(None),
        }
    }

    fn get_pending_balance(&self, address: H160) -> Result<Option<U256>> {
        let balance_key = keys::pending_balance_key(&self.prefix, address);
        let balance: Option<String> = self.conn.get_connection()?.get(balance_key)?;
        let balance = if let Some(s) = balance {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(balance)
    }

    fn get_pending_nonce(&self, address: H160) -> Result<Option<U256>> {
        let nonce_key = keys::pending_nonce_key(&self.prefix, address);
        let nonce: Option<String> = self.conn.get_connection()?.get(nonce_key)?;
        let nonce = if let Some(s) = nonce {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(nonce)
    }

    fn get_pending_byte_code(&self, address: H160) -> Result<Option<Vec<u8>>> {
        let code_key = keys::pending_code_key(&self.prefix, address);
        let code: Option<String> = self.conn.get_connection()?.get(code_key)?;
        let code = if let Some(s) = code {
            Some(hex::decode(s)?)
        } else {
            None
        };
        Ok(code)
    }

    fn get_pending_state(&self, address: H160, index: H256) -> Result<Option<H256>> {
        let state_key = keys::pending_state_key(&self.prefix, address, index);
        let value: Option<String> = self.conn.get_connection()?.get(state_key)?;
        let val = if let Some(s) = value {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(val)
    }

    fn get_total_issuance(&self, height: u32) -> Result<U256> {
        let key = keys::total_issuance_key(&self.prefix);
        let value: Option<String> = self.conn.get_connection()?.vkv_get(key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(val)
    }

    fn get_allowances(&self, height: u32, owner: H160, spender: H160) -> Result<U256> {
        let key = keys::allowances_key(&self.prefix, owner, spender);
        let value: Option<String> = self.conn.get_connection()?.vkv_get(key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(val)
    }
}
