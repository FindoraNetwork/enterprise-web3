use {
    super::{
        storage::StorageKV, EthereumBlockHash, EthereumCurrentBlock, EthereumCurrentReceipts,
        EthereumCurrentTransactionStatuses, DB_KEY_SEPARATOR,
    },
    primitive_types::{H256, U256},
};

impl EthereumBlockHash {
    pub fn build_key(&self, key: &U256) -> Vec<u8> {
        let mut prefix_key = self.prefix();
        let data_key = key.to_string();
        prefix_key.extend_from_slice(DB_KEY_SEPARATOR.as_bytes());
        prefix_key.extend_from_slice(data_key.as_bytes());
        prefix_key
    }
}

impl EthereumCurrentBlock {
    pub fn build_key(&self, key: &H256) -> Vec<u8> {
        let mut prefix_key = self.prefix();
        let data_key = hex::encode_upper(key);
        prefix_key.extend_from_slice(DB_KEY_SEPARATOR.as_bytes());
        prefix_key.extend_from_slice(data_key.as_bytes());
        prefix_key
    }
}

impl EthereumCurrentReceipts {
    pub fn build_key(&self, key: &H256) -> Vec<u8> {
        let mut prefix_key = self.prefix();
        let data_key = hex::encode_upper(key);
        prefix_key.extend_from_slice(DB_KEY_SEPARATOR.as_bytes());
        prefix_key.extend_from_slice(data_key.as_bytes());
        prefix_key
    }
}

impl EthereumCurrentTransactionStatuses {
    pub fn build_key(&self, key: &H256) -> Vec<u8> {
        let mut prefix_key = self.prefix();
        let data_key = hex::encode_upper(key);
        prefix_key.extend_from_slice(DB_KEY_SEPARATOR.as_bytes());
        prefix_key.extend_from_slice(data_key.as_bytes());
        prefix_key
    }
}
