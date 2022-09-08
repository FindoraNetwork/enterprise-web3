use primitive_types::{H160, U256, H256};
use redis::ConnectionLike;
use redis_versioned_kv::VersionedKVCommand;

use crate::{Result, AccountBasic, keys, Error};

pub struct Getter<C> {
    conn: C,
    pub prefix: String,
    pub height: u32,
}

impl<C: ConnectionLike> Getter<C> {
    pub fn new(conn: C, prefix: String) -> Result<Self> {
        let mut s = Self::new_genesis(conn, prefix);

        let height = s.latest_height()?;

        s.height = height;

        Ok(s)
    }

     pub fn new_genesis(conn: C, prefix: String) -> Self {
        Self::new_with_height(conn, prefix, 0)
     }

     pub fn new_with_height(conn: C, prefix: String, height: u32) -> Self {
         Self { conn, prefix, height }
     }

    pub fn latest_height(&mut self) -> Result<u32> {
        Ok(0)
    }

    pub fn get_account_basic(&mut self, address: H160) -> Result<AccountBasic> {
        let balance_key = keys::balance_key(&self.prefix, address);
        let code_key = keys::code_key(&self.prefix, address);
        let nonce_key = keys::nonce_key(&self.prefix, address);

        let height = self.height;

        let balance: Option<String> = self.conn.vkv_get(balance_key, height)?;
        let balance = if let Some(s) = balance {
            U256::from_str_radix(&s, 16)?
        } else {
            U256::zero()
        };

        let code: Option<String> = self.conn.vkv_get(code_key, height)?;
        let code = if let Some(s) = code {
            hex::decode(s)?
        } else {
            Vec::new()
        };

        let nonce: Option<String> = self.conn.vkv_get(nonce_key, height)?;
        let nonce = if let Some(s) = nonce {
            U256::from_str_radix(&s, 16)?
        } else {
            U256::zero()
        };

        Ok(AccountBasic {
            code, nonce, balance
        })
    }

    pub fn get_state(&mut self, address: H160, index: U256) -> Result<H256> {
        let state_key = keys::state_key(&self.prefix, address, index);

        let value: Option<String> = self.conn.vkv_get(state_key, self.height)?;

        let h = if let Some(s) = value {
            let v = hex::decode(s)?;
            if v.len() != 32 {
                return Err(Error::LengthMismatch)
            }

            H256::from_slice(&v)
        } else {
            H256::zero()
        };

        Ok(h)
    }
}

