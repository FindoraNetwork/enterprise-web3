use {
    super::{
        debugapi::{
            debug::DebugApi,
            event_listener::{ContractInfo, DebugEventListener},
            jsvm::func::parse_tracer,
            types::TraceParams,
        },
        internal_err,
    },
    crate::{
        utils::block_number_to_height,
        vm::{precompile::Web3EvmPrecompiles, stack::Web3EvmStackstate},
    },
    chrono::{DateTime, NaiveDateTime, UTC},
    ethereum::{TransactionAction, TransactionV2},
    ethereum_types::{H160, H256, U256},
    evm::{
        executor::stack::{StackExecutor, StackSubstateMetadata},
        CreateScheme::Legacy,
        ExitError, ExitReason,
    },
    evm_exporter::{utils::recover_signer, Getter, PREFIX},
    jsonrpc_core::{Error, Result, Value},
    std::sync::{Arc, Mutex},
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
    mutex: Mutex<bool>,
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
            mutex: Mutex::new(true),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn trace_evm(
        &self,
        from: H160,
        to: Option<H160>,
        value: U256,
        data: Vec<u8>,
        height: u32,
        params: Option<TraceParams>,
        time: DateTime<UTC>,
        block: U256,
        block_hash: H256,
        tx_index: U256,
        tx_hash: H256,
    ) -> Result<Value> {
        let mut lock = self.mutex.try_lock();
        loop {
            if lock.is_ok() {
                break;
            } else {
                lock = self.mutex.try_lock();
            }
        }
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

        let params = params.unwrap_or(TraceParams {
            disable_storage: None,
            disable_memory: None,
            disable_stack: None,
            tracer: None,
            timeout: None,
        });
        let (disable_storage, disable_memory, disable_stack) = {
            (
                params.disable_storage.unwrap_or(false),
                params.disable_memory.unwrap_or(false),
                params.disable_stack.unwrap_or(false),
            )
        };

        let (contract_type, to_addr, input) = match to {
            Some(addr) => (String::from("CALL"), addr, data.clone()),
            None => (
                String::from("CREATE"),
                executor.create_address(Legacy { caller: from }),
                data.clone(),
            ),
        };
        let info = ContractInfo {
            block,
            block_hash,
            tx_index,
            tx_hash,
            contract_type,
            from,
            to: to_addr,
            gas: gas_limit.into(),
            gas_price: self.gas_price.into(),
            input,
            value,
        };
        let funcs = parse_tracer(&params.tracer)?;
        let mut listener = DebugEventListener::new(
            disable_storage,
            disable_memory,
            disable_stack,
            funcs,
            info.clone(),
            U256::from(height),
        );
        if let Some(val) = listener.func.as_ref() {
            let _ = val.call_setup_func(params).map_err(|e| {
                log::error!(target: "debug api", "call_setup_func error:{}",e);
            });
        }

        if let Some(ref func) = listener.func {
            func.call_enter_func(
                info.contract_type.as_str(),
                info.from,
                info.to,
                info.input,
                info.gas,
                info.value,
            )?;
        }

        let gas_used = U256::from(executor.used_gas());
        let mut err = Default::default();
        let mut output = Default::default();
        evm_runtime::tracing::using(&mut listener, || {
            let (exit_reason, ret_val) = match to {
                Some(t) => executor.transact_call(from, t, value, data, gas_limit, access_list),
                None => executor.transact_create(from, value, data, gas_limit, access_list),
            };
            match exit_reason {
                ExitReason::Succeed(_) => err = None,
                ExitReason::Error(e) => match e {
                    ExitError::OutOfGas => err = Some("out of gas".to_string()),
                    _ => err = Some(format!("evm error:{:?}", e)),
                },
                ExitReason::Revert(_) => {
                    if ret_val.len() > 68 {
                        let message_len = ret_val[36..68].iter().sum::<u8>();
                        let body: &[u8] = &ret_val[68..68 + message_len as usize];
                        if let Ok(reason) = std::str::from_utf8(body) {
                            err = Some(format!(
                                "VM Exception while processing transaction: revert {}",
                                reason
                            ));
                        }
                    }
                }
                ExitReason::Fatal(e) => err = Some(format!("evm fatal:{:?}", e)),
            }
            output = ret_val;
        });
        if let Some(ref func) = listener.func {
            func.call_exit_func(gas_used, output.clone(), err)?;
        }
        let gas = U256::from(executor.used_gas());
        listener.get_result(gas, time, output)
    }
}

impl DebugApi for DebugApiImpl {
    fn trace_block_by_number(
        &self,
        number: BlockNumber,
        params: Option<TraceParams>,
    ) -> Result<Vec<Value>> {
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
    ) -> Result<Vec<Value>> {
        log::info!(target: "debug api", "trace_transaction block_hash:{:?}  ", block_hash);
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
        let time = DateTime::<UTC>::from_utc(
            NaiveDateTime::from_timestamp_opt(block.header.timestamp as i64, 0).ok_or({
                let mut err = Error::internal_error();
                err.message = "timestamp out-of-range".to_string();
                err
            })?,
            UTC,
        );
        for (index, tx) in block.transactions.iter().enumerate() {
            let (from, to, value, data) = match tx {
                TransactionV2::Legacy(t) => (
                    recover_signer(t).map_err(|e| {
                        let mut err = Error::internal_error();
                        err.message = format!("{:?}", e);
                        err
                    })?,
                    match t.action {
                        TransactionAction::Call(address) => Some(address),
                        TransactionAction::Create => None,
                    },
                    t.value,
                    t.input.clone(),
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
                    block.header.number.as_u32() - 1,
                    params.clone(),
                    time,
                    block.header.number,
                    block.header.hash(),
                    index.into(),
                    tx.hash(),
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
    ) -> Result<Value> {
        log::info!(target: "debug api", "trace_transaction number:{:?} request:{:?} ", number,request);
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
            UTC::now(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
        .map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })
    }

    fn trace_transaction(&self, tx_hash: H256, params: Option<TraceParams>) -> Result<Value> {
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

        let (from, to, value, data, tx_hash) = match tx {
            TransactionV2::Legacy(ref t) => (
                recover_signer(t).map_err(|e| {
                    let mut err = Error::internal_error();
                    err.message = format!("{:?}", e);
                    err
                })?,
                match t.action {
                    TransactionAction::Call(address) => Some(address),
                    TransactionAction::Create => None,
                },
                t.value,
                t.input.clone(),
                t.hash(),
            ),
            _ => {
                let mut err = Error::internal_error();
                err.message = "tx type not support".to_string();
                return Err(err);
            }
        };

        self.trace_evm(
            from,
            to,
            value,
            data,
            block.header.number.as_u32() - 1,
            params,
            DateTime::<UTC>::from_utc(
                NaiveDateTime::from_timestamp_opt(block.header.timestamp as i64, 0).ok_or({
                    let mut err = Error::internal_error();
                    err.message = "timestamp out-of-range".to_string();
                    err
                })?,
                UTC,
            ),
            block.header.number,
            block.header.hash(),
            index.into(),
            tx_hash,
        )
        .map_err(|e| {
            let mut err = Error::internal_error();
            err.message = format!("{:?}", e);
            err
        })
    }
}
