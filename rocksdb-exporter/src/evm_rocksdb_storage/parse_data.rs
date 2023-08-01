use {
    super::{
        storage_macro::{
            AccountAccountStore, AccountAllowances, AccountTotalIssuance, EVMAccountCodes,
            EVMAccountStorages, EthereumBlockHash, EthereumCurrentBlock,
            EthereumCurrentBlockNumber, EthereumCurrentReceipts,
            EthereumCurrentTransactionStatuses,
        },
        utils::decode_kv,
        Address32, SmartAccount, DB_KEY_SEPARATOR,
    },
    core::str::FromStr,
    ethereum::{BlockV0 as Block, ReceiptV0 as Receipt},
    evm_exporter::TransactionStatus,
    primitive_types::{H160, H256, U256},
    ruc::*,
};

impl AccountAccountStore {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<Option<(H160, (U256, U256))>> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();

        let key = *key_list.get(index + 1).c(d!())?;
        let addr = Address32::from_str(key).c(d!())?;
        let ret = if String::from_utf8_lossy(addr.as_ref()).starts_with("evm:") {
            let smart_account = serde_json::from_slice::<SmartAccount>(&value).c(d!())?;
            Some((
                H160::from_slice(&addr.as_ref()[4..24]),
                (smart_account.nonce, smart_account.balance),
            ))
        } else {
            None
        };
        Ok(ret)
    }
}

impl EthereumCurrentBlockNumber {
    pub fn parse_data(&self, is_decode_kv: bool, kv_pair: &(Box<[u8]>, Box<[u8]>)) -> Result<U256> {
        let val = if is_decode_kv {
            decode_kv(kv_pair).1.to_vec()
        } else {
            kv_pair.1.to_vec()
        };
        serde_json::from_slice::<U256>(&val).c(d!())
    }
}

impl EthereumBlockHash {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<(U256, H256)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();

        let key = *key_list.get(index + 1).c(d!())?;
        Ok((
            U256::from_str(key).c(d!())?,
            serde_json::from_slice::<H256>(&value).c(d!())?,
        ))
    }
}

impl EthereumCurrentBlock {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<(H256, Block)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();
        let key = *key_list.get(index + 1).c(d!())?;
        Ok((
            H256::from_str(key).c(d!())?,
            serde_json::from_slice::<Block>(&value).c(d!())?,
        ))
    }
}

impl EthereumCurrentReceipts {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<(H256, Vec<Receipt>)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();
        let key = *key_list.get(index + 1).c(d!())?;
        Ok((
            H256::from_str(key).c(d!())?,
            serde_json::from_slice::<Vec<Receipt>>(&value).c(d!())?,
        ))
    }
}

impl EthereumCurrentTransactionStatuses {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<(H256, Vec<TransactionStatus>)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();
        let key = *key_list.get(index + 1).c(d!())?;
        Ok((
            H256::from_str(key).c(d!())?,
            serde_json::from_slice::<Vec<TransactionStatus>>(&value).c(d!())?,
        ))
    }
}

impl EVMAccountCodes {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<(H160, Vec<u8>)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();
        let key = *key_list.get(index + 1).c(d!())?;
        Ok((H160::from_str(key).c(d!())?, value))
    }
}

impl EVMAccountStorages {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<((H160, H256), H256)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();

        let key1 = *key_list.get(index + 1).c(d!())?;
        let key2 = *key_list.get(index + 2).c(d!())?;
        Ok((
            (H160::from_str(key1).c(d!())?, H256::from_str(key2).c(d!())?),
            serde_json::from_slice(&value).c(d!())?,
        ))
    }
}

impl AccountTotalIssuance {
    #[allow(unused)]
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<(String, U256)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();

        let key = *key_list.get(index + 1).c(d!())?;

        Ok((String::from(key), serde_json::from_slice(&value).c(d!())?))
    }
}

impl AccountAllowances {
    pub fn parse_data(
        &self,
        is_decode_kv: bool,
        kv_pair: &(Box<[u8]>, Box<[u8]>),
    ) -> Result<((Address32, Address32), U256)> {
        let (key, value) = if is_decode_kv {
            decode_kv(kv_pair)
        } else {
            (kv_pair.0.to_vec(), kv_pair.1.to_vec())
        };
        let key = String::from_utf8_lossy(&key).to_string();
        let mut index = 0;
        if key.starts_with("VER_") {
            index += 2;
        }
        let key_list: Vec<&str> = key.split(DB_KEY_SEPARATOR).collect();

        let key1 = *key_list.get(index + 1).c(d!())?;
        let key2 = *key_list.get(index + 2).c(d!())?;

        let key1 = hex::decode(key1).c(d!())?;
        let key2 = hex::decode(key2).c(d!())?;

        let key1 = TryInto::<[u8; 32]>::try_into(key1).map_err(|_| d!())?;
        let key2 = TryInto::<[u8; 32]>::try_into(key2).map_err(|_| d!())?;

        Ok((
            (Address32(key1), Address32(key2)),
            serde_json::from_slice(&value).c(d!())?,
        ))
    }
}
