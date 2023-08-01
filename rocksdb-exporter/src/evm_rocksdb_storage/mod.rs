use self::storage_macro::AccountTotalIssuance;

mod build_key;
pub mod evm_rocksdb;
mod parse_data;
mod storage;
mod storage_macro;
mod utils;

use {
    self::{
        evm_rocksdb::RocksDB,
        storage::{StorageMap, StorageValue},
        storage_macro::{
            AccountAccountStore, AccountAllowances, EVMAccountCodes, EVMAccountStorages,
            EthereumBlockHash, EthereumCurrentBlock, EthereumCurrentBlockNumber,
            EthereumCurrentReceipts, EthereumCurrentTransactionStatuses,
        },
    },
    bech32::{FromBase32, ToBase32},
    core::{fmt::Formatter, str::FromStr},
    ethereum::{Block, FrontierReceiptData, LegacyTransaction},
    evm_exporter::TransactionStatus,
    primitive_types::{H160, H256, U256},
    ruc::*,
    serde::{Deserialize, Serialize},
    std::sync::Arc,
};

pub const CF_NAME_DEFAULT: &str = "default";
pub const CF_NAME_AUX: &str = "aux";
pub const CF_NAME_STATE: &str = "state";
const DB_KEY_SEPARATOR: &str = "_";
const SPLIT_END: &str = "~";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Address32(pub [u8; 32]);

impl AsRef<[u8]> for Address32 {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}
impl core::fmt::Display for Address32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", bech32::encode("fra", self.to_base32()).unwrap())
    }
}
impl FromStr for Address32 {
    type Err = Box<dyn RucError>;
    fn from_str(s: &str) -> Result<Address32> {
        let d = bech32::decode(s).c(d!())?;
        let v = Vec::<u8>::from_base32(&d.1).c(d!())?;
        let mut address_32 = Address32::default();
        address_32.0.copy_from_slice(v.as_slice());
        Ok(address_32)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SmartAccount {
    pub nonce: U256,
    pub balance: U256,
    pub reserved: U256,
}

pub fn get_current_height(history_db: &Arc<RocksDB>) -> Result<U256> {
    let storage = EthereumCurrentBlockNumber::new(history_db);

    match storage.get(CF_NAME_STATE)? {
        Some(v) => storage.parse_data(false, &v),
        None => Ok(U256::zero()),
    }
}

#[allow(clippy::type_complexity)]
pub fn get_block_info(
    height: U256,
    history_db: &Arc<RocksDB>,
) -> Result<
    Option<(
        Block<LegacyTransaction>,
        Vec<FrontierReceiptData>,
        Vec<TransactionStatus>,
    )>,
> {
    let block_hash_storage = EthereumBlockHash::new(history_db);
    let block_storage = EthereumCurrentBlock::new(history_db);
    let receipts_storage = EthereumCurrentReceipts::new(history_db);
    let status_storage = EthereumCurrentTransactionStatuses::new(history_db);

    let hash = if let Some(kv_pair) =
        block_hash_storage.get(CF_NAME_STATE, &block_hash_storage.build_key(&height))?
    {
        block_hash_storage
            .parse_data(false, &kv_pair)
            .map(|(_, value)| value)?
    } else {
        return Ok(None);
    };

    let block =
        if let Some(kv_pair) = block_storage.get(CF_NAME_STATE, &block_storage.build_key(&hash))? {
            block_storage
                .parse_data(false, &kv_pair)
                .map(|(_, value)| value)?
        } else {
            return Ok(None);
        };

    let receipts = if let Some(kv_pair) =
        receipts_storage.get(CF_NAME_STATE, &receipts_storage.build_key(&hash))?
    {
        receipts_storage
            .parse_data(false, &kv_pair)
            .map(|(_, value)| value)?
    } else {
        return Ok(None);
    };

    let statuses = if let Some(kv_pair) =
        status_storage.get(CF_NAME_STATE, &status_storage.build_key(&hash))?
    {
        status_storage
            .parse_data(false, &kv_pair)
            .map(|(_, value)| value)?
    } else {
        return Ok(None);
    };

    Ok(Some((block, receipts, statuses)))
}

#[allow(clippy::type_complexity)]
pub fn get_account_info(
    state_db: &Arc<RocksDB>,
    height: u64,
) -> Result<(
    Vec<(H160, (U256, U256))>,
    Vec<(H160, Vec<u8>)>,
    Vec<((H160, H256), H256)>,
    Vec<((H160, H160), U256)>,
    U256,
)> {
    let (cf_name, is_decode_kv) = if 0 == height {
        (CF_NAME_DEFAULT, true)
    } else {
        (CF_NAME_AUX, false)
    };
    let mut accountstores = vec![];
    {
        let accountstore_storage = AccountAccountStore::new(state_db);
        for kv_pair in accountstore_storage.get_all(cf_name, true, height)? {
            if let Some(data) = accountstore_storage.parse_data(is_decode_kv, &kv_pair)? {
                accountstores.push(data);
            }
        }
    }

    let mut codes = vec![];
    {
        let code_storage = EVMAccountCodes::new(state_db);
        for kv_pair in code_storage.get_all(cf_name, true, height)? {
            codes.push(code_storage.parse_data(is_decode_kv, &kv_pair)?)
        }
    }

    let mut accounts = vec![];
    {
        let account_storage = EVMAccountStorages::new(state_db);
        for kv_pair in account_storage.get_all(cf_name, true, height)? {
            if kv_pair.1.to_vec().len() <= 1 {
                continue;
            }
            accounts.push(account_storage.parse_data(is_decode_kv, &kv_pair)?)
        }
    }

    let mut allowances = vec![];
    {
        let allowances_storage = AccountAllowances::new(state_db);
        for kv_pair in allowances_storage.get_all(cf_name, true, height)? {
            if let Some(data) = allowances_storage.parse_data(is_decode_kv, &kv_pair)? {
                allowances.push(data);
            }
        }
    }
    let total_issuance_storage = AccountTotalIssuance::new(state_db);
    let total_issuance = if let Some(kv_pair) = total_issuance_storage.get(cf_name)? {
        let (_, value) = total_issuance_storage.parse_data(is_decode_kv, &kv_pair)?;
        value
    } else {
        Default::default()
    };

    Ok((accountstores, codes, accounts, allowances, total_issuance))
}
