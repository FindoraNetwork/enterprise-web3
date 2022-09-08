use primitive_types::U256;

pub struct AccountBasic {
    pub balance: U256,
    pub code: Vec<u8>,
    pub nonce: U256,
}
