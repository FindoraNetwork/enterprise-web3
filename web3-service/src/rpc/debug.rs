use {
    super::{
        debugapi::{
            debug::DebugApi,
            event_listener::DebugEventListener,
            types::{TraceParams, TransactionTrace},
        },
        internal_err,
    },
    crate::{
        utils::block_number_to_height,
        vm::{precompile::Web3EvmPrecompiles, stack::Web3EvmStackstate},
    },
    ethereum::{TransactionAction, TransactionV2},
    ethereum_types::{H160, H256, U256},
    evm::executor::stack::{StackExecutor, StackSubstateMetadata},
    evm_exporter::{utils::recover_signer, Getter, PREFIX},
    jsonrpc_core::{Error, Result},
    std::sync::Arc,
    tendermint_rpc::HttpClient,
    web3_rpc_core::types::{BlockNumber, CallRequest},
};

#[cfg(feature = "cluster_redis")]
pub struct DebugApiImpl {
    chain_id: u32,
    gas_price: u64,
    pool: Arc<r2d2::Pool<redis::cluster::ClusterClient>>,
    tm_client: Arc<HttpClient>,
}
#[cfg(not(feature = "cluster_redis"))]
pub struct DebugApiImpl {
    chain_id: u32,
    gas_price: u64,
    pool: Arc<r2d2::Pool<redis::Client>>,
    tm_client: Arc<HttpClient>,
}
impl DebugApiImpl {
    #[cfg(feature = "cluster_redis")]
    pub fn new(
        chain_id: u32,
        gas_price: u64,
        pool: Arc<r2d2::Pool<redis::cluster::ClusterClient>>,
        tm_client: Arc<HttpClient>,
    ) -> Self {
        Self {
            chain_id,
            gas_price,
            pool,
            tm_client,
        }
    }
    #[cfg(not(feature = "cluster_redis"))]
    pub fn new(
        chain_id: u32,
        gas_price: u64,
        pool: Arc<r2d2::Pool<redis::Client>>,
        tm_client: Arc<HttpClient>,
    ) -> Self {
        Self {
            chain_id,
            gas_price,
            pool,
            tm_client,
        }
    }
    fn trace_evm(
        &self,
        from: H160,
        to: Option<H160>,
        value: U256,
        data: Vec<u8>,
        height: u32,
        params: Option<TraceParams>,
    ) -> Result<TransactionTrace> {
        let gas_limit = U256::from(u32::max_value()).as_u64();
        let config = evm::Config::istanbul();
        let metadata = StackSubstateMetadata::new(gas_limit, &config);
        let precompile_set = Web3EvmPrecompiles::default();
        let mut executor = StackExecutor::new_with_precompiles(
            Web3EvmStackstate::new(
                U256::from(self.gas_price),
                self.chain_id,
                height,
                false,
                from,
                self.pool.clone(),
                self.tm_client.clone(),
                metadata,
            ),
            &config,
            &precompile_set,
        );
        let access_list = Vec::new();
        let (disable_storage, disable_memory, disable_stack) = params
            .map(|p| {
                (
                    p.disable_storage.unwrap_or(false),
                    p.disable_memory.unwrap_or(false),
                    p.disable_stack.unwrap_or(false),
                )
            })
            .unwrap_or((false, false, false));

        let mut listener = DebugEventListener::new(disable_storage, disable_memory, disable_stack);
        evm_runtime::tracing::using(&mut listener, || match to {
            Some(t) => executor.transact_call(from, t, value, data, gas_limit, access_list),
            None => executor.transact_create(from, value, data, gas_limit, access_list),
        });
        Ok(TransactionTrace {
            gas: U256::from(executor.used_gas()),
            return_value: listener.return_value,
            step_logs: listener.step_logs,
        })
    }
}

impl DebugApi for DebugApiImpl {
    fn trace_block_by_number(
        &self,
        number: BlockNumber,
        params: Option<TraceParams>,
    ) -> Result<Vec<TransactionTrace>> {
        let mut conn = self.pool.get().map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let height = match block_number_to_height(Some(number), &mut getter) {
            Ok(h) => h,
            Err(e) => {
                return Err(internal_err(format!(
                    "debug api trace_block_by_number block_number_to_height error:{:?}",
                    e.to_string()
                )));
            }
        };
        let block_hash = match getter.get_block_hash_by_height(U256::from(height)) {
            Ok(value) => {
                if let Some(hash) = value {
                    hash
                } else {
                    return Err(internal_err("block not found"));
                }
            }
            Err(e) => {
                return Err(internal_err(format!(
                    "debug api trace_block_by_number get_block_hash_by_height error:{:?}",
                    e.to_string()
                )));
            }
        };
        self.trace_block_by_hash(block_hash, params)
    }

    fn trace_block_by_hash(
        &self,
        block_hash: H256,
        params: Option<TraceParams>,
    ) -> Result<Vec<TransactionTrace>> {
        let mut conn = self.pool.get().map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let block = getter
            .get_block_by_hash(block_hash)
            .map_err(|e| {
                let mut err = Error::internal_error();
                err.message = format!("{:?}", e);
                err
            })?
            .ok_or({
                let mut err = Error::internal_error();
                err.message = "get_block_by_hash value is none".to_string();
                err
            })?;
        let mut traces = vec![];
        for tx in block.transactions {
            let (from, to, value, data) = match tx {
                TransactionV2::Legacy(t) => (
                    recover_signer(&t).map_err(|e| {
                        let mut err = Error::internal_error();
                        err.message = format!("{:?}", e);
                        err
                    })?,
                    match t.action {
                        TransactionAction::Call(address) => Some(address),
                        TransactionAction::Create => None,
                    },
                    t.value,
                    t.input,
                ),
                _ => {
                    let mut err = Error::internal_error();
                    err.message = "tx type not support".to_string();
                    return Err(err);
                }
            };

            match self
                .trace_evm(
                    from,
                    to,
                    value,
                    data,
                    block.header.number.as_u32(),
                    params.clone(),
                )
                .map_err(|e| {
                    let mut err = Error::internal_error();
                    err.message = format!("{:?}", e);
                    err
                }) {
                Ok(t) => traces.push(t),
                Err(e) => {
                    let mut err = Error::internal_error();
                    err.message = format!("{:?}", e);
                    return Err(err);
                }
            };
        }
        Ok(traces)
    }

    fn trace_call(
        &self,
        request: CallRequest,
        number: BlockNumber,
        params: Option<TraceParams>,
    ) -> Result<TransactionTrace> {
        let mut conn = self.pool.get().map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let height = block_number_to_height(Some(number), &mut getter).map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })?;

        self.trace_evm(
            request.from.unwrap_or_default(),
            request.to,
            request.value.unwrap_or_default(),
            request
                .data
                .ok_or({
                    let mut err = Error::internal_error();
                    err.message = "get transaction value is none".to_string();
                    err
                })?
                .into_vec(),
            height,
            params,
        )
        .map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })
    }

    fn trace_transaction(
        &self,
        tx_hash: H256,
        params: Option<TraceParams>,
    ) -> Result<TransactionTrace> {
        let mut conn = self.pool.get().map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let (block_hash, index) = getter
            .get_transaction_index_by_tx_hash(tx_hash)
            .map_err(|e| {
                let mut err = Error::internal_error();
                err.message = format!("{:?}", e);
                err
            })?
            .ok_or({
                let mut err = Error::internal_error();
                err.message = "get_transaction_index_by_tx_hash value is none".to_string();
                err
            })?;
        let block = getter
            .get_block_by_hash(block_hash)
            .map_err(|e| {
                let mut err = Error::internal_error();
                err.message = format!("{:?}", e);
                err
            })?
            .ok_or({
                let mut err = Error::internal_error();
                err.message = "get_block_by_hash value is none".to_string();
                err
            })?;
        let tx = block
            .transactions
            .get(index as usize)
            .ok_or({
                let mut err = Error::internal_error();
                err.message = "get transaction value is none".to_string();
                err
            })
            .map(|tx| tx.clone())?;

        let (from, to, value, data) = match tx {
            TransactionV2::Legacy(t) => (
                recover_signer(&t).map_err(|e| {
                    let mut err = Error::internal_error();
                    err.message = format!("{:?}", e);
                    err
                })?,
                match t.action {
                    TransactionAction::Call(address) => Some(address),
                    TransactionAction::Create => None,
                },
                t.value,
                t.input,
            ),
            _ => {
                let mut err = Error::internal_error();
                err.message = "tx type not support".to_string();
                return Err(err);
            }
        };

        self.trace_evm(from, to, value, data, block.header.number.as_u32(), params)
            .map_err(|e| {
                let mut err = Error::internal_error();
                err.message = format!("{:?}", e);
                err
            })
    }
}
