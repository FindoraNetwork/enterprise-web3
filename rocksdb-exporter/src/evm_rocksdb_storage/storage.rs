use {super::evm_rocksdb::RocksDB, ruc::*, std::sync::Arc};

pub trait StorageKV {
    fn new(db: &Arc<RocksDB>, decode_kv: bool) -> Self;
    fn prefix(&self) -> Vec<u8>;

    fn get(&self, cf_name: &str, key: &[u8]) -> Result<Option<(Box<[u8]>, Box<[u8]>)>>;

    fn get_all(&self, cf_name: &str, asc: bool, height: u64)
        -> Result<Vec<(Box<[u8]>, Box<[u8]>)>>;
}
pub trait StorageV {
    fn new(db: &Arc<RocksDB>, decode_kv: bool) -> Self;
    fn prefix(&self) -> Vec<u8>;

    fn get(&self, cf_name: &str) -> Result<Option<(Box<[u8]>, Box<[u8]>)>>;
}
