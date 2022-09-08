use ethereum::{BlockAny, BlockV0, BlockV1, BlockV2};
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};

pub const PREFIX: &str = "evm";

pub struct AccountBasic {
    pub balance: U256,
    pub code: Vec<u8>,
    pub nonce: U256,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Block {
    V0(BlockV0),
    V1(BlockV1),
    V2(BlockV2),
    Any(BlockAny),
}

impl Block {
    pub fn hash(&self) -> H256 {
        match self {
            Block::V0(b) => b.header.hash(),
            Block::V1(b) => b.header.hash(),
            Block::V2(b) => b.header.hash(),
            Block::Any(b) => b.header.hash(),
        }
    }

    pub fn time(&self) -> U256 {
        match self {
            Block::V0(b) => U256::from(b.header.timestamp),
            Block::V1(b) => U256::from(b.header.timestamp),
            Block::V2(b) => U256::from(b.header.timestamp),
            Block::Any(b) => U256::from(b.header.timestamp),
        }
    }

    pub fn limit(&self) -> U256 {
        match self {
            Block::V0(b) => b.header.gas_limit,
            Block::V1(b) => b.header.gas_limit,
            Block::V2(b) => b.header.gas_limit,
            Block::Any(b) => b.header.gas_limit,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Block::V0(b) => b.header.number.as_u32(),
            Block::V1(b) => b.header.number.as_u32(),
            Block::V2(b) => b.header.number.as_u32(),
            Block::Any(b) => b.header.number.as_u32(),
        }
    }
}
