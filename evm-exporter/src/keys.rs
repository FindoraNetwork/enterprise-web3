use primitive_types::{H160, U256};

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

pub fn state_key(prefix: &str, addr: H160, index: U256) -> String {
    format!("{}:state:addr.{}:index:u256.{}", prefix, hex::encode(addr), hex_u256(index))
}

pub fn latest_height_key(prefix: &str) -> String {
    format!("{}:height", prefix)
}

