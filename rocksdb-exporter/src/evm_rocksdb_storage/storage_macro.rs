use {
    super::{
        evm_rocksdb::RocksDB,
        storage::{StorageMap, StorageValue},
        DB_KEY_SEPARATOR, SPLIT_END,
    },
    ruc::*,
    std::sync::Arc,
};

macro_rules! generate_value_storage {
    ($module_prefix:ident, $storage_prefix:ident) => {
        paste::paste! {
            pub struct [<$module_prefix $storage_prefix>] {
                db: Arc<RocksDB>,
                module_prefix: String,
                storage_prefix: String,
            }
            impl StorageValue for [<$module_prefix $storage_prefix>] {
                fn new(db: &Arc<RocksDB> ) -> Self {
                    Self {
                        db: db.clone(),
                        module_prefix: stringify!($module_prefix).to_string() ,
                        storage_prefix: stringify!($storage_prefix).to_string(),
                    }
                }

                fn prefix(&self) -> Vec<u8>{
                    let mut prefix_key = self.module_prefix.as_bytes().to_vec();
                    prefix_key.extend_from_slice(DB_KEY_SEPARATOR.as_bytes());
                    prefix_key.extend_from_slice(self.storage_prefix.as_bytes());
                    prefix_key
                }

                fn get(&self, cf_name: &str) -> Result<Option<(Box<[u8]>, Box<[u8]>)>> {
                    let key = self.prefix();

                    if let Some(value) = self.db.get(cf_name, &key)? {
                        Ok(Some((
                            key.into_boxed_slice(),
                            value.into_boxed_slice(),
                        )))
                    } else {
                        Ok(None)
                    }
                }
            }
        }
    };
}

macro_rules! generate_map_storage {
    ($module_prefix:ident, $storage_prefix:ident) => {
        paste::paste! {
            pub struct [<$module_prefix $storage_prefix>] {
                db: Arc<RocksDB>,
                module_prefix: String,
                storage_prefix: String,
            }
            impl StorageMap for [<$module_prefix $storage_prefix>] {
                fn new(db: &Arc<RocksDB>) -> Self {
                    Self {
                        db: db.clone(),
                        module_prefix: stringify!($module_prefix).to_string() ,
                        storage_prefix: stringify!($storage_prefix).to_string(),
                    }
                }
                fn prefix(&self) -> Vec<u8> {
                    [self.module_prefix.as_bytes(), self.storage_prefix.as_bytes(),].concat()
                }

                fn get(&self, cf_name: &str, key: &[u8]) -> Result<Option<(Box<[u8]>, Box<[u8]>)>> {
                    if let Some(value) = self.db.get(cf_name, &key)? {
                        Ok(Some((
                            key.to_vec().into_boxed_slice(),
                            value.into_boxed_slice(),
                        )))
                    } else {
                        Ok(None)
                    }
                }

                fn get_all(&self, cf_name: &str, asc: bool, height: u64) -> Result<Vec<(Box<[u8]>, Box<[u8]>)>> {
                    let prefix_key = if 0 == height {
                        self.prefix()
                    } else {
                        let ver_key = format!("VER{}{:020}{}", DB_KEY_SEPARATOR, height, DB_KEY_SEPARATOR);
                        [ver_key.as_bytes(), self.prefix().into_boxed_slice().as_ref()].concat()
                    };

                    let mut lower = prefix_key.to_vec();
                    lower.extend_from_slice(DB_KEY_SEPARATOR.as_bytes());
                    let mut upper = prefix_key.to_vec();
                    upper.extend_from_slice(SPLIT_END.as_bytes());

                    let mut data = vec![];
                    for kv_pair in self.db.iterate(
                        &lower,
                        &upper,
                        asc,
                        cf_name,
                    )? {
                        data.push(kv_pair);
                    }
                    Ok(data)
                }
            }
        }
    };
}

generate_value_storage!(Ethereum, CurrentBlockNumber);
generate_value_storage!(Account, TotalIssuance);
generate_map_storage!(Account, AccountStore);
generate_map_storage!(Ethereum, BlockHash);
generate_map_storage!(Ethereum, CurrentBlock);
generate_map_storage!(Ethereum, CurrentReceipts);
generate_map_storage!(Ethereum, CurrentTransactionStatuses);
generate_map_storage!(EVM, AccountCodes);
generate_map_storage!(EVM, AccountStorages);
generate_map_storage!(Account, Allowances);
