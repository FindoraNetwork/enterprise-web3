mod precompile;

use crate::utils::block_number_to_height;
use crate::vm::precompile::PRECOMPILE_SET;
use evm::backend::{Backend, Basic};
use evm::executor::stack::{MemoryStackState, PrecompileSet, StackExecutor, StackSubstateMetadata};
use evm::ExitReason;
use evm_exporter::Block;
use evm_exporter::{Getter, PREFIX};
use log::error;
use once_cell::sync::Lazy;
// use ovr_ruc::{alt, min};
use ovr_ruc::*;
use primitive_types::{H160, H256, U256};
use redis::{Client, Commands, Connection, ConnectionLike};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use web3_rpc_core::types::{BlockNumber, CallRequest};

#[derive(Debug)]
pub struct EthVmBackend {
    gas_price: U256,
    cli: Client,
    upstream: String,
    chain_id: u32,
    rollback_height: Option<u32>,
    pub height_hash_map: HashMap<u32, H256>,
    pub hash_height_map: HashMap<H256, u32>,
}

pub enum ConfigType {
    Frontier,
    Istanbul,
    Berlin,
    London,
}

impl Clone for EthVmBackend {
    fn clone(&self) -> Self {
        Self {
            gas_price: self.gas_price,
            cli: self.cli.clone(),
            upstream: self.upstream.clone(),
            chain_id: self.chain_id,
            rollback_height: self.rollback_height.clone(),
            height_hash_map: Default::default(),
            hash_height_map: Default::default(),
        }
    }
}

impl EthVmBackend {
    pub fn new(gas_price: u64, redis_addr: &str, upstream: &str, chain_id: u32) -> Result<Self> {
        let cli = Client::open(redis_addr).c(d!())?;
        let mut eb = Self {
            gas_price: U256::from(gas_price),
            cli,
            upstream: upstream.to_string(),
            chain_id,
            rollback_height: None,
            height_hash_map: HashMap::new(),
            hash_height_map: HashMap::new(),
        };
        eb.load_block_height_hash_map().c(d!())?;
        Ok(eb)
    }

    fn load_block_height_hash_map(&mut self) -> Result<()> {
        let current_block = self.get_block_by_number(None).c(d!())?;
        let current_height = current_block.height();
        let current_hash = current_block.hash();

        let mut m1 = HashMap::new();
        let mut m2 = HashMap::new();

        m1.insert(current_height, current_hash);
        m2.insert(current_hash, current_height);

        for i in (0..current_height).rev() {
            let block = self.get_block_by_number(Some(i)).c(d!())?;
            let height = block.height();
            let hash = block.hash();

            m1.insert(height, hash);
            m2.insert(hash, height);
        }

        self.height_hash_map = m1;
        self.hash_height_map = m2;

        Ok(())
    }

    pub fn contract_handle(
        &self,
        req: CallRequest,
        bn: Option<BlockNumber>,
        ct: Option<ConfigType>,
    ) -> Result<(ExitReason, Vec<u8>)> {
        static U64_MAX: Lazy<U256> = Lazy::new(|| U256::from(u64::MAX));

        // Operation Type
        enum Operation {
            Call,
            Create,
        }

        // Determine what type of operation is being performed based on the parameter to in the request object
        let (operation, address) = if let Some(to) = req.to {
            (Operation::Call, to)
        } else {
            (Operation::Create, H160::default())
        };

        let caller = req.from.unwrap_or_default();
        let value = req.value.unwrap_or_default();
        let data = req.data.unwrap_or_default();

        // This parameter is used as the divisor and cannot be 0
        let gas = if let Some(gas) = req.gas {
            alt!(gas > *U64_MAX, u64::MAX, gas.as_u64())
        } else {
            u64::MAX
        };
        let gas_price = req.gas_price.unwrap_or_else(U256::one);
        let gas_price = alt!(gas_price > *U64_MAX, u64::MAX, gas_price.as_u64());
        let gas_limit = gas.checked_div(gas_price).unwrap(); //safe

        // If the gas_limit is too large, the function may not return for a long time
        let gas_limit = min!(gas_limit, 1_000_000_000);

        let cfg = if ct.is_none() {
            evm::Config::istanbul()
        } else {
            match ct.unwrap() {
                ConfigType::Frontier => evm::Config::frontier(),
                ConfigType::Istanbul => evm::Config::istanbul(),
                ConfigType::Berlin => evm::Config::berlin(),
                ConfigType::London => evm::Config::london(),
            }
        };

        let metadata = StackSubstateMetadata::new(u64::MAX, &cfg);

        let rollback_height = block_number_to_height(bn, self).c(d!())?;
        let mut backend = self.clone();
        backend.rollback_height = Some(rollback_height);
        let stack = MemoryStackState::new(metadata, &backend);
        let precompiles = PRECOMPILE_SET.clone();
        let mut executor = StackExecutor::new_with_precompiles(stack, &cfg, &precompiles);

        let resp = match operation {
            Operation::Call => {
                executor.transact_call(caller, address, value, data.0, gas_limit, vec![])
            }
            Operation::Create => executor.transact_create(caller, value, data.0, gas_limit, vec![]),
        };

        Ok(resp)
    }

    pub fn gen_getter(&self, height: Option<u32>) -> Result<Getter<Connection>> {
        let con = self.cli.get_connection().c(d!())?;
        if let Some(h) = height {
            Ok(Getter::new_with_height(con, PREFIX.to_string(), h))
        } else {
            Getter::new(con, PREFIX.to_string()).c(d!())
        }
    }

    pub fn get_block_by_number(&self, height: Option<u32>) -> Result<Block> {
        let height = if let Some(h) = height {
            h
        } else {
            let getter = self.gen_getter(None).c(d!())?;
            getter.height
        };

        let mut con = self.cli.get_connection().c(d!())?;
        let block_key = format!("block:{}", height);
        let val: Option<String> = con.get(block_key).c(d!())?;
        if let Some(val) = val {
            let block = serde_json::from_str::<Block>(&val).c(d!())?;
            Ok(block)
        } else {
            Err(eg!())
        }
    }

    pub fn get_block_proposer(&self, height: Option<u32>) -> Result<H160> {
        let height = if let Some(h) = height {
            h
        } else {
            let getter = self.gen_getter(None).c(d!())?;
            getter.height
        };

        let url = format!("{}/block?height={}", self.upstream, height);
        let resp = reqwest::blocking::get(url)
            .c(d!())?
            .json::<Value>()
            .c(d!())?;
        if let Some(proposer_address) =
            resp["result"]["block"]["header"]["proposer_address"].as_str()
        {
            Ok(H160::from_str(proposer_address).c(d!())?)
        } else {
            Err(eg!())
        }
    }

    fn select_height(&self, num: Option<U256>) -> u32 {
        if let Some(h) = num {
            h.as_u32()
        } else {
            if let Some(rh) = self.rollback_height {
                rh
            } else {
                self.gen_getter(None)
                    .map_err(|e| error!("{:?}", e))
                    .map(|g| g.height)
                    .unwrap_or_default()
            }
        }
    }
}

impl Backend for EthVmBackend {
    fn gas_price(&self) -> U256 {
        self.gas_price
    }

    //TODO tx from? or genesis tx from?
    fn origin(&self) -> H160 {
        todo!()
    }

    fn block_hash(&self, number: U256) -> H256 {
        let height = self.select_height(Some(number));

        self.get_block_by_number(Some(height))
            .map_err(|e| error!("{:?}", e))
            .map(|b| b.hash())
            .unwrap_or_default()
    }

    fn block_number(&self) -> U256 {
        let height = self.select_height(None);
        U256::from(height)
    }

    fn block_coinbase(&self) -> H160 {
        let height = self.select_height(None);
        self.get_block_proposer(Some(height))
            .map_err(|e| error!("{:?}", e))
            .unwrap_or_default()
    }

    fn block_timestamp(&self) -> U256 {
        let height = self.select_height(None);
        self.get_block_by_number(Some(height))
            .map_err(|e| error!("{:?}", e))
            .map(|b| b.time())
            .unwrap_or_default()
    }

    //TODO need impl?
    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }

    fn block_gas_limit(&self) -> U256 {
        let height = self.select_height(None);
        self.get_block_by_number(Some(height))
            .map_err(|e| error!("{:?}", e))
            .map(|b| b.limit())
            .unwrap_or_default()
    }

    //TODO need impl?
    fn block_base_fee_per_gas(&self) -> U256 {
        U256::zero()
    }

    fn chain_id(&self) -> U256 {
        U256::from(self.chain_id)
    }

    fn exists(&self, address: H160) -> bool {
        let height = self.select_height(None);
        if let Ok(mut getter) = self.gen_getter(Some(height)) {
            getter
                .get_account_basic(address)
                .map_err(|e| error!("{:?}", e))
                .map(|ab| {
                    if ab.nonce == U256::zero() || ab.balance == U256::zero() {
                        false
                    } else {
                        true
                    }
                })
                .unwrap_or_default()
        } else {
            false
        }
    }

    fn basic(&self, address: H160) -> Basic {
        let height = self.select_height(None);
        if let Ok(mut getter) = self.gen_getter(Some(height)) {
            getter
                .get_account_basic(address)
                .map_err(|e| error!("{:?}", e))
                .map(|ab| Basic {
                    balance: ab.balance,
                    nonce: ab.nonce,
                })
                .unwrap_or_default()
        } else {
            Basic::default()
        }
    }

    fn code(&self, address: H160) -> Vec<u8> {
        let height = self.select_height(None);
        if let Ok(mut getter) = self.gen_getter(Some(height)) {
            getter
                .get_account_basic(address)
                .map_err(|e| error!("{:?}", e))
                .map(|ab| ab.code)
                .unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        let height = self.select_height(None);
        if let Ok(mut getter) = self.gen_getter(Some(height)) {
            getter
                .get_state(address, index)
                .map_err(|e| error!("{:?}", e))
                .unwrap_or_default()
        } else {
            H256::zero()
        }
    }

    //TODO need impl?
    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        None
    }
}
