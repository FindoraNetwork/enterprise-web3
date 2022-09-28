mod build_key;
pub mod evm_rocksdb;
mod parse_data;
mod storage;
mod storage_macro;
mod utils;

use {
    self::{
        evm_rocksdb::RocksDB,
        storage::{StorageKV, StorageV},
        storage_macro::{
            AccountAccountStore, EVMAccountCodes, EVMAccountStorages, EthereumBlockHash,
            EthereumCurrentBlock, EthereumCurrentBlockNumber, EthereumCurrentReceipts,
            EthereumCurrentTransactionStatuses,
        },
    },
    ethereum::{Block, FrontierReceiptData, LegacyTransaction},
    evm_exporter::TransactionStatus,
    primitive_types::{H160, H256, U256},
    ruc::*,
    std::sync::Arc,
};

pub const CF_NAME_DEFAULT: &str = "default";
pub const CF_NAME_AUX: &str = "aux";
pub const CF_NAME_STATE: &str = "state";
const DB_KEY_SEPARATOR: &str = "_";
const SPLIT_BGN: &str = "_";
const SPLIT_END: &str = "~";

pub fn get_current_height(history_db: &Arc<RocksDB>) -> Result<U256> {
    let storage = EthereumCurrentBlockNumber::new(&history_db);

    match storage.get(CF_NAME_STATE)? {
        Some(v) => storage.parse_data(false, &v),
        None => Ok(U256::zero()),
    }
}

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

pub fn get_account_info(
    state_db: &Arc<RocksDB>,
    height: u64,
) -> Result<(
    Vec<(H160, (U256, U256))>,
    Vec<(H160, Vec<u8>)>,
    Vec<((H160, H256), H256)>,
)> {
    let (cf_name, is_decode_kv) = if 0 == height {
        (CF_NAME_DEFAULT, true)
    } else {
        (CF_NAME_AUX, false)
    };
    let accountstore_storage = AccountAccountStore::new(state_db);
    let mut accountstores = vec![];
    for kv_pair in accountstore_storage.get_all(cf_name, true, height)? {
        if let Some(data) = accountstore_storage.parse_data(is_decode_kv, &kv_pair)? {
            accountstores.push(data);
        }
    }

    let code_storage = EVMAccountCodes::new(state_db);
    let mut codes = vec![];
    for kv_pair in code_storage.get_all(cf_name, true, height)? {
        codes.push(code_storage.parse_data(is_decode_kv, &kv_pair)?)
    }

    let account_storage = EVMAccountStorages::new(state_db);
    let mut account_storages = vec![];
    for kv_pair in account_storage.get_all(cf_name, true, height)? {
        if kv_pair.1.to_vec().len() <= 1 {
            continue;
        }
        account_storages.push(account_storage.parse_data(is_decode_kv, &kv_pair)?)
    }

    Ok((accountstores, codes, account_storages))
}
