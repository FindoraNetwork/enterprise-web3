use {
    crate::{keys, AccountBasic, Block, Receipt, Result, TransactionStatus},
    primitive_types::{H160, H256, U256},
    redis::{Commands, ConnectionLike},
    redis_versioned_kv::VersionedKVCommand,
};

pub struct Getter<'a, C> {
    conn: &'a mut C,
    pub prefix: String,
}

impl<'a, C: ConnectionLike> Getter<'a, C> {
    pub fn new(conn: &'a mut C, prefix: String) -> Self {
        Self { conn, prefix }
    }

    pub fn latest_height(&mut self) -> Result<u32> {
        let height_key = keys::latest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get(height_key)?;
        match height {
            Some(str) => Ok(str.parse::<u32>()?),
            _ => Ok(0),
        }
    }
    pub fn lowest_height(&mut self) -> Result<u32> {
        let height_key = keys::lowest_height_key(&self.prefix);
        let height: Option<String> = self.conn.get(height_key)?;
        match height {
            Some(str) => Ok(str.parse::<u32>()?),
            _ => Ok(0),
        }
    }
    pub fn get_balance(&mut self, height: u32, address: H160) -> Result<U256> {
        let balance_key = keys::balance_key(&self.prefix, address);
        let balance: Option<String> = self.conn.vkv_get(balance_key, height)?;
        let balance = if let Some(s) = balance {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(balance)
    }

    pub fn get_nonce(&mut self, height: u32, address: H160) -> Result<U256> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        let nonce: Option<String> = self.conn.vkv_get(nonce_key, height)?;
        let nonce = if let Some(s) = nonce {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(nonce)
    }

    pub fn get_byte_code(&mut self, height: u32, address: H160) -> Result<Vec<u8>> {
        let code_key = keys::code_key(&self.prefix, address);
        let code: Option<String> = self.conn.vkv_get(code_key, height)?;
        let code = if let Some(s) = code {
            hex::decode(s)?
        } else {
            Vec::new()
        };
        Ok(code)
    }

    pub fn get_account_basic(&mut self, height: u32, address: H160) -> Result<AccountBasic> {
        Ok(AccountBasic {
            balance: self.get_balance(height, address)?,
            code: self.get_byte_code(height, address)?,
            nonce: self.get_nonce(height, address)?,
        })
    }

    pub fn addr_state_exists(&mut self, height: u32, address: H160) -> Result<bool> {
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        let value: Option<String> = self.conn.vkv_get(state_addr_key, height)?;
        Ok(value.is_some())
    }

    pub fn get_state(&mut self, height: u32, address: H160, index: H256) -> Result<H256> {
        let state_key = keys::state_key(&self.prefix, address, index);
        let value: Option<String> = self.conn.vkv_get(state_key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            H256::zero()
        };
        Ok(val)
    }

    pub fn get_block_hash_by_height(&mut self, height: U256) -> Result<Option<H256>> {
        let block_hash_key = keys::block_hash_key(&self.prefix, height);
        let value: Option<String> = self.conn.get::<&str, Option<String>>(&block_hash_key)?;
        if let Some(hash) = value {
            Ok(Some(serde_json::from_str(hash.as_str())?))
        } else {
            Ok(None)
        }
    }

    pub fn get_height_by_block_hash(&mut self, block_hash: H256) -> Result<Option<U256>> {
        let block_height_key = keys::block_height_key(&self.prefix, block_hash);
        let value: Option<String> = self.conn.get::<&str, Option<String>>(&block_height_key)?;
        if let Some(hash) = value {
            Ok(Some(serde_json::from_str(hash.as_str())?))
        } else {
            Ok(None)
        }
    }

    pub fn get_block_by_hash(&mut self, block_hash: H256) -> Result<Option<Block>> {
        let block_key = keys::block_key(&self.prefix, block_hash);
        let value: Option<String> = self.conn.get::<&str, Option<String>>(&block_key)?;
        if let Some(block) = value {
            Ok(Some(serde_json::from_str(block.as_str())?))
        } else {
            Ok(None)
        }
    }

    pub fn get_transaction_receipt_by_block_hash(
        &mut self,
        block_hash: H256,
    ) -> Result<Option<Vec<Receipt>>> {
        let receipt_key = keys::receipt_key(&self.prefix, block_hash);
        let value: Option<String> = self.conn.get(receipt_key)?;

        match value {
            Some(receipts) => Ok(serde_json::from_str(receipts.as_str())?),
            _ => Ok(None),
        }
    }

    pub fn get_transaction_status_by_block_hash(
        &mut self,
        block_hash: H256,
    ) -> Result<Option<Vec<TransactionStatus>>> {
        let status_key = keys::status_key(&self.prefix, block_hash);

        let value: Option<String> = self.conn.get(status_key)?;

        match value {
            Some(statuses) => Ok(serde_json::from_str(statuses.as_str())?),
            _ => Ok(None),
        }
    }

    pub fn get_transaction_index_by_tx_hash(
        &mut self,
        tx_hash: H256,
    ) -> Result<Option<(H256, u32)>> {
        let transaction_index_key = keys::transaction_index_key(&self.prefix, tx_hash);

        let value: Option<String> = self.conn.get(transaction_index_key)?;

        match value {
            Some(hash_index) => Ok(serde_json::from_str(hash_index.as_str())?),
            _ => Ok(None),
        }
    }

    pub fn get_pending_balance(&mut self, address: H160) -> Result<Option<U256>> {
        let balance_key = keys::pending_balance_key(&self.prefix, address);
        let balance: Option<String> = self.conn.get(balance_key)?;
        let balance = if let Some(s) = balance {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(balance)
    }

    pub fn get_pending_nonce(&mut self, address: H160) -> Result<Option<U256>> {
        let nonce_key = keys::pending_nonce_key(&self.prefix, address);
        let nonce: Option<String> = self.conn.get(nonce_key)?;
        let nonce = if let Some(s) = nonce {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(nonce)
    }

    pub fn get_pending_byte_code(&mut self, address: H160) -> Result<Option<Vec<u8>>> {
        let code_key = keys::pending_code_key(&self.prefix, address);
        let code: Option<String> = self.conn.get(code_key)?;
        let code = if let Some(s) = code {
            Some(hex::decode(s)?)
        } else {
            None
        };
        Ok(code)
    }

    pub fn get_pending_state(&mut self, address: H160, index: H256) -> Result<Option<H256>> {
        let state_key = keys::pending_state_key(&self.prefix, address, index);
        let value: Option<String> = self.conn.get(state_key)?;
        let val = if let Some(s) = value {
            Some(serde_json::from_str(s.as_str())?)
        } else {
            None
        };
        Ok(val)
    }

    pub fn get_total_issuance(&mut self, height: u32) -> Result<U256> {
        let key = keys::total_issuance_key(&self.prefix);
        let value: Option<String> = self.conn.vkv_get(key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(val)
    }

    pub fn get_allowances(&mut self, height: u32, owner: &[u8], spender: &[u8]) -> Result<U256> {
        let key = keys::allowances_key(&self.prefix, owner, spender);
        let value: Option<String> = self.conn.vkv_get(key, height)?;
        let val = if let Some(s) = value {
            serde_json::from_str(s.as_str())?
        } else {
            U256::zero()
        };
        Ok(val)
    }
}
