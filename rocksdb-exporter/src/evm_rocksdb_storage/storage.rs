use {super::evm_rocksdb::RocksDB, ruc::*, std::sync::Arc};

pub trait StorageValue {
    fn new(db: &Arc<RocksDB>) -> Self;
    fn prefix(&self) -> Vec<u8>;

    #[allow(clippy::type_complexity)]
    fn get(&self, cf_name: &str) -> Result<Option<(Box<[u8]>, Box<[u8]>)>>;
}

pub trait StorageMap {
    fn new(db: &Arc<RocksDB>) -> Self;
    fn prefix(&self) -> Vec<u8>;

    #[allow(clippy::type_complexity)]
    fn get(&self, cf_name: &str, key: &[u8]) -> Result<Option<(Box<[u8]>, Box<[u8]>)>>;

    #[allow(clippy::type_complexity)]
    fn get_all(&self, cf_name: &str, asc: bool, height: u64)
        -> Result<Vec<(Box<[u8]>, Box<[u8]>)>>;
}
