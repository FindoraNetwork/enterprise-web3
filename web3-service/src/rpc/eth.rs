use crate::vm::EthVmBackend;
use ethereum_types::{Address, H160, H256, H64, U256, U64};
use evm::backend::{Backend, Basic, MemoryBackend};
use evm::executor::stack::MemoryStackState;
use evm_exporter::Getter;
use jsonrpc_core::*;
use redis::{Client, ConnectionLike};
use std::sync::Arc;
use tokio::sync::Mutex;
use web3_rpc_core::types::*;
use web3_rpc_core::EthApi;

pub struct EthService {
    eth_vm_backend: EthVmBackend,
}

impl EthService {
    pub fn new(
        gas_price: u64,
        redis_addr: &str,
        upstream: &str,
        chain_id: u32,
    ) -> anyhow::Result<Self> {
        let backend = EthVmBackend::new(gas_price, redis_addr, upstream, chain_id)?;
        Ok(Self {
            eth_vm_backend: backend,
        })
    }
}

impl EthApi for EthService {
    fn protocol_version(&self) -> BoxFuture<Result<u64>> {
        todo!()
    }

    fn hashrate(&self) -> Result<U256> {
        todo!()
    }

    fn chain_id(&self) -> BoxFuture<Result<Option<U64>>> {
        todo!()
    }

    fn accounts(&self) -> Result<Vec<H160>> {
        todo!()
    }

    fn balance(&self, _: H160, _: Option<BlockNumber>) -> BoxFuture<Result<U256>> {
        todo!()
    }

    fn send_transaction(&self, _: TransactionRequest) -> BoxFuture<Result<H256>> {
        todo!()
    }

    fn call(&self, _: CallRequest, _: Option<BlockNumber>) -> BoxFuture<Result<Bytes>> {
        todo!()
    }

    fn syncing(&self) -> BoxFuture<Result<SyncStatus>> {
        todo!()
    }

    fn author(&self) -> BoxFuture<Result<H160>> {
        todo!()
    }

    fn is_mining(&self) -> BoxFuture<Result<bool>> {
        todo!()
    }

    fn gas_price(&self) -> BoxFuture<Result<U256>> {
        todo!()
    }

    fn block_number(&self) -> BoxFuture<Result<U256>> {
        todo!()
    }

    fn storage_at(&self, _: H160, _: U256, _: Option<BlockNumber>) -> BoxFuture<Result<H256>> {
        todo!()
    }

    fn block_by_hash(&self, _: H256, _: bool) -> BoxFuture<Result<Option<RichBlock>>> {
        todo!()
    }

    fn block_by_number(&self, _: BlockNumber, _: bool) -> BoxFuture<Result<Option<RichBlock>>> {
        todo!()
    }

    fn transaction_count(&self, _: H160, _: Option<BlockNumber>) -> BoxFuture<Result<U256>> {
        todo!()
    }

    fn block_transaction_count_by_hash(&self, _: H256) -> BoxFuture<Result<Option<U256>>> {
        todo!()
    }

    fn block_transaction_count_by_number(&self, _: BlockNumber) -> BoxFuture<Result<Option<U256>>> {
        todo!()
    }

    fn block_uncles_count_by_hash(&self, _: H256) -> Result<U256> {
        todo!()
    }

    fn block_uncles_count_by_number(&self, _: BlockNumber) -> Result<U256> {
        todo!()
    }

    fn code_at(&self, _: H160, _: Option<BlockNumber>) -> BoxFuture<Result<Bytes>> {
        todo!()
    }

    fn send_raw_transaction(&self, _: Bytes) -> BoxFuture<Result<H256>> {
        todo!()
    }

    fn estimate_gas(&self, _: CallRequest, _: Option<BlockNumber>) -> BoxFuture<Result<U256>> {
        todo!()
    }

    fn transaction_by_hash(&self, _: H256) -> BoxFuture<Result<Option<Transaction>>> {
        todo!()
    }

    fn transaction_by_block_hash_and_index(
        &self,
        _: H256,
        _: Index,
    ) -> BoxFuture<Result<Option<Transaction>>> {
        todo!()
    }

    fn transaction_by_block_number_and_index(
        &self,
        _: BlockNumber,
        _: Index,
    ) -> BoxFuture<Result<Option<Transaction>>> {
        todo!()
    }

    fn transaction_receipt(&self, _: H256) -> BoxFuture<Result<Option<Receipt>>> {
        todo!()
    }

    fn uncle_by_block_hash_and_index(&self, _: H256, _: Index) -> Result<Option<RichBlock>> {
        todo!()
    }

    fn uncle_by_block_number_and_index(
        &self,
        _: BlockNumber,
        _: Index,
    ) -> Result<Option<RichBlock>> {
        todo!()
    }

    fn logs(&self, _: Filter) -> BoxFuture<Result<Vec<Log>>> {
        todo!()
    }

    fn work(&self) -> Result<Work> {
        todo!()
    }

    fn submit_work(&self, _: H64, _: H256, _: H256) -> Result<bool> {
        todo!()
    }

    fn submit_hashrate(&self, _: U256, _: H256) -> Result<bool> {
        todo!()
    }
}
