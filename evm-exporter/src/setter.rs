use {
    crate::{
        error::Result,
        keys,
        types::{Block, TransactionStatus},
        utils::recover_signer,
        Receipt,
    },
    ethereum::LegacyTransaction,
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

    pub fn clear(&mut self) -> Result<()> {
        redis::cmd("FLUSHDB").arg("SYNC").query(&mut self.conn)?;
        Ok(())
    }

    pub fn set_height(&mut self, height: u32) -> Result<()> {
        let height_key = keys::latest_height_key(&self.prefix);
        self.conn.set(height_key, format!("{}", height))?;
        Ok(())
    }

    pub fn set_balance(&mut self, height: u32, address: H160, balance: U256) -> Result<()> {
        let balance_key = keys::balance_key(&self.prefix, address);
        self.conn
            .vkv_set(balance_key, height, serde_json::to_string(&balance)?)?;

        Ok(())
    }

    pub fn set_nonce(&mut self, height: u32, address: H160, nonce: U256) -> Result<()> {
        let nonce_key = keys::nonce_key(&self.prefix, address);
        self.conn
            .vkv_set(nonce_key, height, serde_json::to_string(&nonce)?)?;

        Ok(())
    }

    pub fn set_byte_code(&mut self, height: u32, address: H160, code: Vec<u8>) -> Result<()> {
        let code_key = keys::code_key(&self.prefix, address);
        self.conn.vkv_set(code_key, height, hex::encode(&code))?;

        Ok(())
    }

    pub fn set_state(
        &mut self,
        height: u32,
        address: H160,
        index: H256,
        value: H256,
    ) -> Result<()> {
        let key = keys::state_key(&self.prefix, address, index);
        self.conn
            .vkv_set(key, height, serde_json::to_string(&value)?)?;
        let state_addr_key = keys::state_addr_key(&self.prefix, address);
        self.conn
            .vkv_set(state_addr_key.clone(), height, state_addr_key)?;
        Ok(())
    }

    pub fn set_block_info(
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

        for (i, tx) in block.transactions.iter().enumerate() {
            let transaction_index_key = keys::transaction_index_key(&self.prefix, tx.hash());
            self.conn.set(
                transaction_index_key,
                serde_json::to_string(&(block_hash, i as u32))?,
            )?;
        }
        Ok(())
    }

    pub fn set_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()> {
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

    pub fn set_pending_code(&mut self, address: H160, code: Vec<u8>) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn
            .set(pending_code_key, serde_json::to_string(&code)?)?;
        Ok(())
    }

    pub fn set_pending_state(&mut self, address: H160, index: H256, value: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn
            .set(pending_state_key, serde_json::to_string(&value)?)?;

        Ok(())
    }

    pub fn remove_pending_tx(&mut self, transaction: LegacyTransaction) -> Result<()> {
        let sign_address = recover_signer(&transaction)?;
        let pending_balance_key = keys::pending_balance_key(&self.prefix, sign_address);

        self.conn.del(pending_balance_key)?;

        let pending_nonce_key = keys::pending_nonce_key(&self.prefix, sign_address);
        self.conn.del(pending_nonce_key)?;

        Ok(())
    }

    pub fn remove_pending_code(&mut self, address: H160) -> Result<()> {
        let pending_code_key = keys::pending_code_key(&self.prefix, address);
        self.conn.del(pending_code_key)?;
        Ok(())
    }

    pub fn remove_pending_state(&mut self, address: H160, index: H256) -> Result<()> {
        let pending_state_key = keys::pending_state_key(&self.prefix, address, index);
        self.conn.del(pending_state_key)?;

        Ok(())
    }
}
