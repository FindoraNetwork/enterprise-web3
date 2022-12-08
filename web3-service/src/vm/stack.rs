use {
    ethereum::Log,
    ethereum_types::{H160, H256, U256},
    evm::{
        backend::{Backend, Basic},
        executor::stack::{Accessed, StackState, StackSubstateMetadata},
    },
    evm_exporter::{Getter, PREFIX},
    jsonrpc_core::Value,
    ruc::{d, eg, RucResult},
    std::{
        borrow::Cow,
        collections::{BTreeMap, BTreeSet},
        mem::swap,
        str::FromStr,
        sync::Arc,
    },
};
#[cfg(feature = "cluster_redis")]
pub struct Web3EvmStackstate<'config> {
    gas_price: U256,
    chain_id: u32,
    height: u32,
    origin: H160,
    pool: Arc<r2d2::Pool<redis::cluster::ClusterClient>>,
    tendermint_url: String,
    metadata: StackSubstateMetadata<'config>,
    deletes: BTreeSet<H160>,
    parent: Option<Box<Self>>,
    code: BTreeMap<H160, Vec<u8>>,
    storage: BTreeMap<(H160, H256), H256>,
    transfer_balance: BTreeMap<H160, U256>,
    pub logs: Vec<Log>,
}
#[cfg(not(feature = "cluster_redis"))]
pub struct Web3EvmStackstate<'config> {
    gas_price: U256,
    chain_id: u32,
    height: u32,
    is_pending: bool,
    origin: H160,
    pool: Arc<r2d2::Pool<redis::Client>>,
    tendermint_url: String,
    metadata: StackSubstateMetadata<'config>,
    deletes: BTreeSet<H160>,
    parent: Option<Box<Self>>,
    code: BTreeMap<H160, Vec<u8>>,
    storage: BTreeMap<(H160, H256), H256>,
    transfer_balance: BTreeMap<H160, U256>,
    pub logs: Vec<Log>,
}
impl<'config> Web3EvmStackstate<'config> {
    #[cfg(feature = "cluster_redis")]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gas_price: U256,
        chain_id: u32,
        height: u32,
        is_pending: bool,
        origin: H160,
        pool: Arc<r2d2::Pool<redis::cluster::ClusterClient>>,
        tendermint_url: &str,
        metadata: StackSubstateMetadata<'config>,
    ) -> Self {
        Self {
            gas_price,
            chain_id,
            height,
            is_pending,
            origin,
            metadata,
            pool,
            tendermint_url: tendermint_url.into(),
            deletes: BTreeSet::new(),
            parent: None,
            code: BTreeMap::new(),
            storage: BTreeMap::new(),
            transfer_balance: BTreeMap::new(),
            logs: vec![],
        }
    }
    #[cfg(not(feature = "cluster_redis"))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gas_price: U256,
        chain_id: u32,
        height: u32,
        is_pending: bool,
        origin: H160,
        pool: Arc<r2d2::Pool<redis::Client>>,
        tendermint_url: &str,
        metadata: StackSubstateMetadata<'config>,
    ) -> Self {
        Self {
            gas_price,
            chain_id,
            height,
            is_pending,
            origin,
            metadata,
            pool,
            tendermint_url: tendermint_url.into(),
            deletes: BTreeSet::new(),
            parent: None,
            code: BTreeMap::new(),
            storage: BTreeMap::new(),
            transfer_balance: BTreeMap::new(),
            logs: vec![],
        }
    }
    pub fn recursive_is_cold<F: Fn(&Accessed) -> bool>(&self, f: &F) -> bool {
        let local_is_accessed = self.metadata.accessed().as_ref().map(f).unwrap_or(false);
        if local_is_accessed {
            false
        } else {
            self.parent
                .as_ref()
                .map(|p| p.recursive_is_cold(f))
                .unwrap_or(true)
        }
    }
}
impl<'config> Backend for Web3EvmStackstate<'config> {
    fn gas_price(&self) -> U256 {
        self.gas_price
    }

    fn origin(&self) -> H160 {
        self.origin
    }

    fn block_hash(&self, height: U256) -> H256 {
        let func = || -> ruc::Result<H256> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
            getter
                .get_block_hash_by_height(height)
                .c(d!())?
                .ok_or(eg!())
        };
        func().unwrap_or_default()
    }

    fn block_number(&self) -> U256 {
        let func = || -> ruc::Result<U256> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
            getter.latest_height().c(d!()).map(U256::from)
        };
        func().unwrap_or_default()
    }

    fn block_coinbase(&self) -> H160 {
        let func = || -> ruc::Result<H160> {
            attohttpc::post(format!(
                "{}/block?height={}",
                self.tendermint_url, self.height
            ))
            .header(attohttpc::header::CONTENT_TYPE, "application/json")
            .send()
            .c(d!())
            .and_then(|resp| resp.json::<Value>().c(d!()))
            .and_then(|json_resp| {
                H160::from_str(
                    json_resp["result"]["block"]["header"]["proposer_address"]
                        .as_str()
                        .ok_or(eg!())?,
                )
                .c(d!())
            })
        };
        func().unwrap_or_default()
    }

    fn block_timestamp(&self) -> U256 {
        let func = || -> ruc::Result<U256> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
            let block_hash = getter
                .get_block_hash_by_height(U256::from(self.height))
                .c(d!())?
                .ok_or(eg!())?;
            getter
                .get_block_by_hash(block_hash)
                .c(d!())?
                .ok_or(eg!())
                .map(|b| U256::from(b.header.timestamp))
        };
        func().unwrap_or_default()
    }

    fn block_difficulty(&self) -> U256 {
        let func = || -> ruc::Result<U256> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
            let block_hash = getter
                .get_block_hash_by_height(U256::from(self.height))
                .c(d!())?
                .ok_or(eg!())?;
            getter
                .get_block_by_hash(block_hash)
                .c(d!())?
                .ok_or(eg!())
                .map(|b| b.header.difficulty)
        };
        func().unwrap_or_default()
    }

    fn block_gas_limit(&self) -> U256 {
        let func = || -> ruc::Result<U256> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
            let block_hash = getter
                .get_block_hash_by_height(U256::from(self.height))
                .c(d!())?
                .ok_or(eg!())?;
            getter
                .get_block_by_hash(block_hash)
                .c(d!())?
                .ok_or(eg!())
                .map(|b| b.header.gas_limit)
        };
        func().unwrap_or_default()
    }

    fn block_base_fee_per_gas(&self) -> U256 {
        U256::from(100_0000_0000_u64)
    }

    fn chain_id(&self) -> U256 {
        U256::from(self.chain_id)
    }

    fn exists(&self, address: H160) -> bool {
        let func = || -> ruc::Result<bool> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
            let balance = getter.get_balance(self.height, address).c(d!())?;
            let nonce = getter.get_nonce(self.height, address).c(d!())?;
            Ok(nonce != U256::zero() && balance != U256::zero())
        };
        func().unwrap_or_default()
    }

    fn basic(&self, address: H160) -> Basic {
        let func = || -> ruc::Result<Basic> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());

            let balance = if self.is_pending {
                getter
                    .get_pending_balance(address)
                    .c(d!())?
                    .unwrap_or(getter.get_balance(self.height, address).c(d!())?)
            } else {
                getter.get_balance(self.height, address).c(d!())?
            };

            let nonce = if self.is_pending {
                getter
                    .get_pending_nonce(address)
                    .c(d!())?
                    .unwrap_or(getter.get_nonce(self.height, address).c(d!())?)
            } else {
                getter.get_nonce(self.height, address).c(d!())?
            };

            Ok(Basic { balance, nonce })
        };
        func().unwrap_or_default()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        if let Some(value) = self.code.get(&address) {
            return value.to_vec();
        }
        let func = || -> ruc::Result<Vec<u8>> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());

            Ok(if self.is_pending {
                getter
                    .get_pending_byte_code(address)
                    .c(d!())?
                    .unwrap_or(getter.get_byte_code(self.height, address).c(d!())?)
            } else {
                getter.get_byte_code(self.height, address).c(d!())?
            })
        };
        func().unwrap_or_default()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        if let Some(value) = self.storage.get(&(address, index)) {
            return *value;
        }

        let func = || -> ruc::Result<H256> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());

            Ok(if self.is_pending {
                getter
                    .get_pending_state(address, index)
                    .c(d!())?
                    .unwrap_or(getter.get_state(self.height, address, index).c(d!())?)
            } else {
                getter.get_state(self.height, address, index).c(d!())?
            })
        };
        func().unwrap_or_default()
    }

    fn original_storage(&self, _: H160, _: H256) -> Option<H256> {
        None
    }
}

impl<'config> StackState<'config> for Web3EvmStackstate<'config> {
    fn metadata(&self) -> &StackSubstateMetadata<'config> {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
        &mut self.metadata
    }

    fn enter(&mut self, gas_limit: u64, is_static: bool) {
        let mut entering = Self::new(
            self.gas_price,
            self.chain_id,
            self.height,
            self.is_pending,
            self.origin,
            self.pool.clone(),
            &self.tendermint_url,
            self.metadata.spit_child(gas_limit, is_static),
        );
        entering.deletes = self.deletes.clone();
        entering.code = self.code.clone();
        entering.storage = self.storage.clone();
        entering.transfer_balance = self.transfer_balance.clone();
        entering.logs = self.logs.clone();
        swap(&mut entering, self);

        self.parent = Some(Box::new(entering));
    }

    fn exit_commit(&mut self) -> Result<(), evm::ExitError> {
        let mut exited = *self
            .parent
            .take()
            .ok_or_else(|| evm::ExitError::Other(Cow::from("Cannot commit on root substate")))?;
        exited.deletes = self.deletes.clone();
        exited.code = self.code.clone();
        exited.storage = self.storage.clone();
        exited.transfer_balance = self.transfer_balance.clone();
        exited.logs = self.logs.clone();
        swap(&mut exited, self);

        self.metadata.swallow_commit(exited.metadata)?;
        self.logs.append(&mut exited.logs);
        self.deletes.append(&mut exited.deletes);
        Ok(())
    }

    fn exit_revert(&mut self) -> Result<(), evm::ExitError> {
        let mut exited = *self
            .parent
            .take()
            .ok_or_else(|| evm::ExitError::Other(Cow::from("Cannot revert on root substate")))?;
        exited.deletes = self.deletes.clone();
        exited.code = self.code.clone();
        exited.storage = self.storage.clone();
        exited.transfer_balance = self.transfer_balance.clone();
        exited.logs = self.logs.clone();
        swap(&mut exited, self);
        self.metadata.swallow_revert(exited.metadata)?;
        Ok(())
    }

    fn exit_discard(&mut self) -> Result<(), evm::ExitError> {
        let mut exited = *self
            .parent
            .take()
            .ok_or_else(|| evm::ExitError::Other(Cow::from("Cannot discard on root substate")))?;
        exited.deletes = self.deletes.clone();
        exited.code = self.code.clone();
        exited.storage = self.storage.clone();
        exited.transfer_balance = self.transfer_balance.clone();
        exited.logs = self.logs.clone();
        swap(&mut exited, self);
        self.metadata.swallow_discard(exited.metadata)?;
        Ok(())
    }

    fn is_empty(&self, address: H160) -> bool {
        let func = || -> ruc::Result<bool> {
            let mut conn = self.pool.get().c(d!())?;
            let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
            let balance = getter.get_balance(self.height, address).c(d!())?;
            let nonce = getter.get_nonce(self.height, address).c(d!())?;
            let code = getter.get_byte_code(self.height, address).c(d!())?;
            Ok(nonce == U256::zero() && balance == U256::zero() && code.len() == 0)
        };
        func().unwrap_or_default()
    }

    fn deleted(&self, address: H160) -> bool {
        if self.deletes.contains(&address) {
            return true;
        }
        if let Some(parent) = self.parent.as_ref() {
            return parent.deleted(address);
        }
        false
    }

    fn is_cold(&self, address: H160) -> bool {
        self.recursive_is_cold(&|a| a.accessed_addresses.contains(&address))
    }

    fn is_storage_cold(&self, address: H160, key: H256) -> bool {
        self.recursive_is_cold(&|a: &Accessed| a.accessed_storage.contains(&(address, key)))
    }

    fn inc_nonce(&mut self, _: H160) {}

    fn set_storage(&mut self, address: H160, index: H256, value: H256) {
        self.storage.insert((address, index), value);
    }

    fn reset_storage(&mut self, address: H160) {
        let mut keys = vec![];
        for (addr, index) in self.storage.keys() {
            if address == *addr {
                keys.push((*addr, *index));
            }
        }
        keys.iter().for_each(|key| {
            self.storage.remove(key);
        });
    }

    fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
        self.logs.push(Log {
            address,
            topics,
            data,
        });
    }

    fn set_deleted(&mut self, address: H160) {
        self.deletes.insert(address);
    }

    fn set_code(&mut self, address: H160, code: Vec<u8>) {
        self.code.insert(address, code);
    }

    fn transfer(&mut self, transfer: evm::Transfer) -> Result<(), evm::ExitError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|_| evm::ExitError::Other(Cow::from("redis connect error")))?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());

        let default_source_balance = getter
            .get_balance(self.height, transfer.source)
            .map_err(|_| evm::ExitError::Other(Cow::from("get source balance error")))?;

        let source_balance = self
            .transfer_balance
            .get(&transfer.source)
            .unwrap_or(&default_source_balance);

        let source_balance = source_balance
            .checked_sub(transfer.value)
            .c(d!("insufficient balance"))
            .map_err(|_| evm::ExitError::OutOfFund)?;

        let default_target_balance = getter
            .get_balance(self.height, transfer.target)
            .map_err(|_| evm::ExitError::Other(Cow::from("get target balance error")))?;

        let target_balance = self
            .transfer_balance
            .get(&transfer.source)
            .unwrap_or(&default_target_balance);

        let target_balance = target_balance
            .checked_sub(transfer.value)
            .c(d!("balance overflow"))
            .map_err(|_| evm::ExitError::OutOfFund)?;

        self.transfer_balance
            .insert(transfer.source, source_balance);

        self.transfer_balance
            .insert(transfer.target, target_balance);
        Ok(())
    }

    fn reset_balance(&mut self, _: H160) {}

    fn touch(&mut self, _: H160) {}
}
