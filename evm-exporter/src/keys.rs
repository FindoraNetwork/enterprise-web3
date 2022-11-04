use primitive_types::{H160, H256, U256};

pub fn balance_key(prefix: &str, addr: H160) -> String {
    format!("{}:balance:addr.{}", prefix, hex::encode(addr))
}

pub fn code_key(prefix: &str, addr: H160) -> String {
    format!("{}:code:addr.{}", prefix, hex::encode(addr))
}

pub fn nonce_key(prefix: &str, addr: H160) -> String {
    format!("{}:nonce:addr.{}", prefix, hex::encode(addr))
}

pub fn hex_u256(u256: U256) -> String {
    format!("{:#064x}", u256)
}
pub fn state_addr_key(prefix: &str, addr: H160) -> String {
    format!("{}:state:addr.{}", prefix, hex::encode(addr),)
}
pub fn state_key(prefix: &str, addr: H160, index: H256) -> String {
    format!(
        "{}:state:addr.{}:index:u256.{}",
        prefix,
        hex::encode(addr),
        hex::encode(index)
    )
}

pub fn latest_height_key(prefix: &str) -> String {
    format!("{}:height", prefix)
}

pub fn tx_state_key(prefix: &str, tx_hash: H256) -> String {
    format!("{}:tx_state:hash.{}", prefix, hex::encode(tx_hash))
}

pub fn block_hash_key(prefix: &str, height: U256) -> String {
    format!("{}:block_hash:height.{:?}", prefix, hex_u256(height))
}

pub fn block_height_key(prefix: &str, block_hash: H256) -> String {
    format!("{}:block_height:hash.{:?}", prefix, hex::encode(block_hash))
}

pub fn block_key(prefix: &str, block_hash: H256) -> String {
    format!("{}:block:block_hash.{:?}", prefix, hex::encode(block_hash))
}

pub fn receipt_key(prefix: &str, tx_hash: H256) -> String {
    format!("{}:receipt:tx_hash.{}", prefix, hex::encode(tx_hash))
}

pub fn status_key(prefix: &str, block_hash: H256) -> String {
    format!("{}:status:block_hash.{:?}", prefix, hex::encode(block_hash))
}

pub fn transaction_index_key(prefix: &str, tx_hash: H256) -> String {
    format!("{}:tx_index_key:tx_hash.{:?}", prefix, hex::encode(tx_hash))
}

pub fn pending_balance_key(prefix: &str, addr: H160) -> String {
    format!("{}:pending_balance:addr.{}", prefix, hex::encode(addr))
}

pub fn pending_nonce_key(prefix: &str, addr: H160) -> String {
    format!("{}:pending_nonce:addr.{}", prefix, hex::encode(addr))
}

pub fn pending_code_key(prefix: &str, addr: H160) -> String {
    format!("{}:pending_code:addr.{}", prefix, hex::encode(addr))
}

pub fn pending_state_key(prefix: &str, addr: H160, index: H256) -> String {
    format!(
        "{}:pending_state:addr.{}:index:u256.{}",
        prefix,
        hex::encode(addr),
        hex::encode(index)
    )
}
