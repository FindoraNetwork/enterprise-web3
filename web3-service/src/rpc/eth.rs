use {
    super::internal_err,
    crate::{
        utils::block_number_to_height,
        vm::{
            evm_tx::{EvmRawTxWrapper, UncheckedTransaction},
            precompile::Web3EvmPrecompiles,
            stack::Web3EvmStackstate,
        },
    },
    ethereum::{LegacyTransaction, ReceiptAny, TransactionAny, TransactionV2},
    ethereum_types::{BigEndianHash, H160, H256, H512, H64, U256, U64},
    evm::{
        executor::stack::{StackExecutor, StackSubstateMetadata},
        ExitError, ExitReason,
    },
    evm_exporter::{public_key, Getter, TransactionStatus},
    futures::TryFutureExt,
    jsonrpc_core::{futures::future, BoxFuture, Error, ErrorCode, Result, Value},
    lazy_static::lazy_static,
    sha3::{Digest, Keccak256},
    std::{
        collections::BTreeMap,
        sync::{mpsc, Arc},
    },
    tendermint::abci::Code,
    tendermint_rpc::{Client, HttpClient},
    tokio::runtime::Runtime,
    web3_rpc_core::{
        types::{
            Block, BlockNumber, BlockTransactions, Bytes, CallRequest, Filter, FilteredParams,
            Index, Log, Receipt, RichBlock, SyncStatus, Transaction, TransactionRequest, Work,
        },
        EthApi,
    },
};

const MAX_PAST_LOGS: u32 = 10000;
lazy_static! {
    static ref RT: Runtime = Runtime::new().expect("Failed to create thread pool executor");
}
pub struct EthService {
    chain_id: u32,
    gas_price: u64,
    getter: Arc<dyn Getter + Sync + Send>,
    tm_client: Arc<HttpClient>,
    tendermint_url: String,
}

impl EthService {
    pub fn new(
        chain_id: u32,
        gas_price: u64,
        getter: Arc<dyn Getter + Sync + Send>,
        tm_client: Arc<HttpClient>,
        tendermint_url: &str,
    ) -> Self {
        Self {
            chain_id,
            gas_price,
            getter,
            tm_client,
            tendermint_url: tendermint_url.into(),
        }
    }

    fn transaction_build(
        block: &evm_exporter::Block,
        transaction: &TransactionAny,
        status: &TransactionStatus,
    ) -> Transaction {
        if let TransactionV2::Legacy(tx) = transaction {
            Transaction {
                hash: status.transaction_hash,
                nonce: tx.nonce,
                block_hash: Some(block.header.hash()),
                block_number: Some(block.header.number),
                transaction_index: Some(U256::from(status.transaction_index)),
                from: status.from,
                to: status.to,
                value: tx.value,
                gas_price: tx.gas_price,
                gas: tx.gas_limit,
                input: Bytes(tx.input.clone()),
                creates: status.contract_address,
                raw: Bytes(rlp::encode(tx).to_vec()),
                public_key: public_key(tx).ok().map(H512::from),
                chain_id: tx.signature.chain_id().map(U64::from),
                standard_v: U256::from(tx.signature.standard_v()),
                v: U256::from(tx.signature.v()),
                r: U256::from(tx.signature.r().as_bytes()),
                s: U256::from(tx.signature.s().as_bytes()),
            }
        } else {
            Transaction::default()
        }
    }
    fn rich_block_build(
        block: &evm_exporter::Block,
        statuses: Vec<TransactionStatus>,
        full_transactions: bool,
    ) -> RichBlock {
        RichBlock {
            inner: Block {
                hash: Some(block.header.hash()),
                parent_hash: block.header.parent_hash,
                uncles_hash: block.header.ommers_hash,
                author: block.header.beneficiary,
                miner: block.header.beneficiary,
                state_root: block.header.state_root,
                transactions_root: block.header.transactions_root,
                receipts_root: block.header.receipts_root,
                number: Some(block.header.number),
                gas_used: block.header.gas_used,
                gas_limit: block.header.gas_limit,
                extra_data: Bytes(block.header.extra_data.clone()),
                logs_bloom: Some(block.header.logs_bloom),
                timestamp: U256::from(block.header.timestamp),
                difficulty: block.header.difficulty,
                total_difficulty: U256::zero(),
                seal_fields: vec![
                    Bytes(block.header.mix_hash.as_bytes().to_vec()),
                    Bytes(block.header.nonce.as_bytes().to_vec()),
                ],
                uncles: vec![],
                transactions: {
                    if full_transactions {
                        BlockTransactions::Full(
                            block
                                .transactions
                                .iter()
                                .enumerate()
                                .map(|(index, transaction)| {
                                    if let Some(status) = statuses.get(index) {
                                        Self::transaction_build(block, transaction, status)
                                    } else {
                                        Transaction::default()
                                    }
                                })
                                .collect(),
                        )
                    } else {
                        BlockTransactions::Hashes(
                            block
                                .transactions
                                .iter()
                                .map(|tx| {
                                    let data = match tx {
                                        TransactionAny::Legacy(t) => rlp::encode(t),
                                        TransactionAny::EIP2930(t) => rlp::encode(t),
                                        TransactionAny::EIP1559(t) => rlp::encode(t),
                                    };
                                    H256::from_slice(Keccak256::digest(&data).as_slice())
                                })
                                .collect(),
                        )
                    }
                },
                size: Some(U256::from(rlp::encode(block).len() as u32)),
            },
            extra_info: BTreeMap::new(),
        }
    }

    fn error_on_execution_failure(
        reason: &ExitReason,
        data: &[u8],
    ) -> std::result::Result<(), Error> {
        match reason {
            ExitReason::Succeed(_) => Ok(()),
            ExitReason::Error(e) => {
                if *e == ExitError::OutOfGas {
                    return Err(Error {
                        code: ErrorCode::ServerError(0),
                        message: "out of gas".to_string(),
                        data: None,
                    });
                }
                Err(Error {
                    code: ErrorCode::InternalError,
                    message: format!("evm error: {:?}", e),
                    data: Some(Value::String("0x".to_string())),
                })
            }
            ExitReason::Revert(_) => {
                let mut message = "VM Exception while processing transaction: revert".to_string();
                if data.len() > 68 {
                    let message_len = data[36..68].iter().sum::<u8>();
                    let body: &[u8] = &data[68..68 + message_len as usize];
                    if let Ok(reason) = std::str::from_utf8(body) {
                        message = format!("{} {}", message, reason);
                    }
                }
                Err(Error {
                    code: ErrorCode::InternalError,
                    message,
                    data: Some(serde_json::value::Value::String(hex::encode(data))),
                })
            }
            ExitReason::Fatal(e) => Err(Error {
                code: ErrorCode::InternalError,
                message: format!("evm fatal: {:?}", e),
                data: Some(Value::String("0x".to_string())),
            }),
        }
    }
}

impl EthApi for EthService {
    fn protocol_version(&self) -> BoxFuture<Result<u64>> {
        log::info!(target: "eth api", "protocol_version");
        Box::pin(future::ok(1))
    }

    fn hashrate(&self) -> Result<U256> {
        log::info!(target: "eth api", "hashrate");
        Ok(U256::zero())
    }

    fn chain_id(&self) -> BoxFuture<Result<Option<U64>>> {
        log::info!(target: "eth api", "chain_id");
        Box::pin(future::ok(Some(U64::from(self.chain_id))))
    }

    fn accounts(&self) -> Result<Vec<H160>> {
        log::info!(target: "eth api", "accounts");
        Ok(Vec::new())
    }

    fn balance(&self, address: H160, number: Option<BlockNumber>) -> BoxFuture<Result<U256>> {
        log::info!(target: "eth api", "balance address:{:?} number:{:?}", &address, &number);

        let getter = self.getter.clone();
        let address_clone = address.clone();
        let number_clone = number.clone();

        Box::pin(async move {
            let getter_clone = getter.clone();
            if let Some(BlockNumber::Pending) = number_clone {
                let result = tokio::task::spawn_blocking(move || {
                    getter_clone.get_pending_balance(address_clone)
                })
                .await
                .map_err(|e| {
                    internal_err(format!(
                        "eth api balance get_pending_balance error: {:?}",
                        e.to_string()
                    ))
                })?;

                match result {
                    Ok(Some(balance)) => return Ok(balance),
                    Ok(None) => {}
                    Err(e) => {
                        return Err(internal_err(format!(
                            "eth api balance get_pending_balance error: {:?}",
                            e.to_string()
                        )))
                    }
                }
            }

            let getter_clone = getter.clone();
            let height = tokio::task::spawn_blocking(move || {
                block_number_to_height(number_clone, getter_clone)
            })
            .await
            .map_err(|e| {
                internal_err(format!(
                    "eth api balance block_number_to_height error: {:?}",
                    e.to_string()
                ))
            })?
            .map_err(|e| {
                internal_err(format!(
                    "eth api balance block_number_to_height error: {:?}",
                    e.to_string()
                ))
            })?;

            let result = tokio::task::spawn_blocking(move || getter.get_balance(height, address))
                .await
                .map_err(|e| {
                    internal_err(format!(
                        "eth api balance get_balance error: {:?}",
                        e.to_string()
                    ))
                })?;

            match result {
                Ok(balance) => Ok(balance),
                Err(e) => Err(internal_err(format!(
                    "eth api balance get_balance error: {:?}",
                    e.to_string()
                ))),
            }
        })
    }

    fn send_transaction(&self, _: TransactionRequest) -> BoxFuture<Result<H256>> {
        log::info!(target: "eth api", "send_transaction");
        let mut err = Error::method_not_found();
        err.message = String::from("send_transaction is disabled");
        Box::pin(future::err(err))
    }

    fn call(&self, request: CallRequest, number: Option<BlockNumber>) -> BoxFuture<Result<Bytes>> {
        log::info!(target: "eth api", "call request:{:?} number:{:?}", &request, &number);
        let is_pending = matches!(number, Some(BlockNumber::Pending));

        let height = match block_number_to_height(number, self.getter.clone()) {
            Ok(h) => h,
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api call block_number_to_height error:{:?}",
                    e.to_string()
                ))));
            }
        };

        let gas_limit = U256::from(u32::max_value()).as_u64();
        let data = request.data.map(|d| d.0).unwrap_or_default();
        let config = evm::Config::istanbul();

        let metadata = StackSubstateMetadata::new(gas_limit, &config);
        let precompile_set = Web3EvmPrecompiles::new(height);
        let mut executor = StackExecutor::new_with_precompiles(
            Web3EvmStackstate::new(
                U256::from(self.gas_price),
                self.chain_id,
                height,
                is_pending,
                request.from.unwrap_or_default(),
                self.getter.clone(),
                self.tendermint_url.as_str(),
                metadata,
            ),
            &config,
            &precompile_set,
        );
        let access_list = Vec::new();

        if let Some(to) = request.to {
            let (_, retv) = executor.transact_call(
                request.from.unwrap_or_default(),
                to,
                request.value.unwrap_or_default(),
                data,
                gas_limit,
                access_list,
            );
            Box::pin(future::ok(Bytes(retv)))
        } else {
            let err = jsonrpc_core::Error {
                code: jsonrpc_core::ErrorCode::InvalidParams,
                message: "to address no find".to_string(),
                data: None,
            };
            Box::pin(future::err(err))
        }
    }

    fn syncing(&self) -> BoxFuture<Result<SyncStatus>> {
        log::info!(target: "eth api", "syncing");
        Box::pin(future::ok(SyncStatus::None))
    }

    fn author(&self) -> BoxFuture<Result<H160>> {
        log::info!(target: "eth api", "author");
        let getter = self.getter.clone();

        Box::pin(async move {
            let getter_clone = getter.clone();
            let height = tokio::task::spawn_blocking(move || getter_clone.latest_height())
                .await
                .map_err(|e| {
                    internal_err(format!(
                        "eth api author latest_height error: {:?}",
                        e.to_string()
                    ))
                })?
                .map_err(|e| {
                    internal_err(format!(
                        "eth api author latest_height error: {:?}",
                        e.to_string()
                    ))
                })?;

            let getter_clone = getter.clone();
            let hash = tokio::task::spawn_blocking(move || {
                getter_clone.get_block_hash_by_height(U256::from(height))
            })
            .await
            .map_err(|e| {
                internal_err(format!(
                    "eth api author get_block_hash_by_height error: {:?}",
                    e.to_string()
                ))
            })?
            .unwrap_or_else(|_| Some(H256::default()));

            // hash always has a default value so unwrap here is safe
            let hash = hash.unwrap();

            let block =
                match tokio::task::spawn_blocking(move || getter.get_block_by_hash(hash)).await {
                    Ok(Ok(Some(value))) => value,

                    Ok(Ok(None)) => {
                        return Ok(H160::default());
                    }

                    Ok(Err(e)) => {
                        return Err(internal_err(format!(
                            "eth api author get_block_by_hash error: {:?}",
                            e.to_string()
                        )));
                    }

                    Err(e) => {
                        return Err(internal_err(format!(
                            "eth api spawn_blocking error: {:?}",
                            e.to_string()
                        )));
                    }
                };

            Ok(block.header.beneficiary)
        })
    }

    fn is_mining(&self) -> BoxFuture<Result<bool>> {
        log::info!(target: "eth api", "is_mining");
        Box::pin(future::ok(false))
    }

    fn gas_price(&self) -> BoxFuture<Result<U256>> {
        log::info!(target: "eth api", "gas_price");
        Box::pin(future::ok(U256::from(self.gas_price)))
    }

    fn block_number(&self) -> BoxFuture<Result<U256>> {
        log::info!(target: "eth api", "block_number");
        let getter = self.getter.clone();

        Box::pin(async move {
            let result = tokio::task::spawn_blocking(move || getter.latest_height())
                .await
                .map_err(|e| {
                    internal_err(format!(
                        "eth api block_number latest_height error:{:?}",
                        e.to_string()
                    ))
                })?;

            match result {
                Ok(height) => Ok(U256::from(height)),
                Err(e) => Err(internal_err(format!(
                    "eth api block_number latest_height error:{:?}",
                    e.to_string()
                ))),
            }
        })
    }

    fn storage_at(
        &self,
        address: H160,
        index: U256,
        number: Option<BlockNumber>,
    ) -> BoxFuture<Result<H256>> {
        log::info!(target: "eth api", "storage_at address:{:?} index:{:?} number:{:?}", &address, &index, &number);
        let getter = self.getter.clone();
        let index = H256::from_uint(&index);

        Box::pin(async move {
            if let Some(BlockNumber::Pending) = number {
                let getter_clone = getter.clone();
                let pending_state = tokio::task::spawn_blocking(move || {
                    getter_clone.get_pending_state(address, index)
                })
                .await
                .map_err(|e| {
                    internal_err(format!(
                        "eth api storage_at spawn_blocking get_pending_state error: {:?}",
                        e.to_string()
                    ))
                })?;

                match pending_state {
                    Ok(Some(value)) => return Ok(value),
                    Ok(None) => return Ok(H256::default()),
                    Err(e) => {
                        return Err(internal_err(format!(
                            "eth api storage_at get_pending_state error: {:?}",
                            e.to_string()
                        )));
                    }
                }
            }

            let getter_clone = getter.clone();
            let height = tokio::task::spawn_blocking(move || {
                block_number_to_height(number, getter_clone.clone())
            })
            .await
            .map_err(|e| {
                internal_err(format!(
                    "eth api storage_at spawn_blocking block_number_to_height error: {:?}",
                    e.to_string()
                ))
            })?
            .map_err(|e| {
                internal_err(format!(
                    "eth api storage_at block_number_to_height error: {:?}",
                    e.to_string()
                ))
            })?;

            let state =
                tokio::task::spawn_blocking(move || getter.get_state(height, address, index))
                    .await
                    .map_err(|e| {
                        internal_err(format!(
                            "eth api storage_at spawn_blocking get_state error: {:?}",
                            e.to_string()
                        ))
                    })?;

            match state {
                Ok(value) => Ok(value),
                Err(e) => Err(internal_err(format!(
                    "eth api storage_at get_state error: {:?}",
                    e.to_string()
                ))),
            }
        })
    }

    fn block_by_hash(&self, hash: H256, full: bool) -> BoxFuture<Result<Option<RichBlock>>> {
        log::info!(target: "eth api", "block_by_hash hash:{:?} full:{:?}", &hash, &full);
        let getter = self.getter.clone();

        Box::pin(async move {
            let getter_clone = getter.clone();
            let block_result =
                tokio::task::spawn_blocking(move || getter_clone.get_block_by_hash(hash))
                    .await
                    .map_err(|e| {
                        internal_err(format!(
                            "eth api block_by_hash spawn_blocking get_block_by_hash error: {:?}",
                            e.to_string()
                        ))
                    })?;

            let block = match block_result {
                Ok(Some(value)) => value,
                Ok(None) => return Ok(None),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api block_by_hash get_block_by_hash error: {:?}",
                        e.to_string()
                    )))
                }
            };

            let transaction_statuses_result = tokio::task::spawn_blocking(move || getter.get_transaction_status_by_block_hash(hash))
            .await
            .map_err(|e| internal_err(format!(
                "eth api block_by_hash spawn_blocking get_transaction_status_by_block_hash error: {:?}", e.to_string()
            )))?;

            let transaction_statuses = match transaction_statuses_result {
                Ok(Some(value)) => value,
                Ok(None) => return Ok(None),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api block_by_hash get_transaction_status_by_block_hash error: {:?}",
                        e.to_string()
                    )))
                }
            };

            Ok(Some(Self::rich_block_build(
                &block,
                transaction_statuses,
                full,
            )))
        })
    }

    fn block_by_number(
        &self,
        number: BlockNumber,
        full: bool,
    ) -> BoxFuture<Result<Option<RichBlock>>> {
        log::info!(target: "eth api", "block_by_number number:{:?} full:{:?}", &number, &full);
        let getter = self.getter.clone();

        Box::pin(async move {
            let height = match block_number_to_height(Some(number), getter.clone()) {
                Ok(h) => h,
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api block_by_number block_number_to_height error: {:?}",
                        e.to_string()
                    )))
                }
            };

            let getter_clone = getter.clone();
            let hash_result = tokio::task::spawn_blocking(move || {
                getter_clone.get_block_hash_by_height(U256::from(height))
            })
            .await
            .map_err(|e| {
                internal_err(format!(
                    "eth api block_by_number spawn_blocking get_block_hash_by_height error: {:?}",
                    e.to_string()
                ))
            })?;

            let hash = match hash_result {
                Ok(Some(value)) => value,
                Ok(None) => return Ok(None),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api block_by_number get_block_hash_by_height error: {:?}",
                        e.to_string()
                    )))
                }
            };

            let getter_clone = getter.clone();
            let block_result =
                tokio::task::spawn_blocking(move || getter_clone.get_block_by_hash(hash))
                    .await
                    .map_err(|e| {
                        internal_err(format!(
                            "eth api block_by_number spawn_blocking get_block_by_hash error: {:?}",
                            e.to_string()
                        ))
                    })?;

            let block = match block_result {
                Ok(Some(value)) => value,
                Ok(None) => return Ok(None),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api block_by_number get_block_by_hash error: {:?}",
                        e.to_string()
                    )))
                }
            };

            let transaction_statuses_result = tokio::task::spawn_blocking(move || getter.get_transaction_status_by_block_hash(hash))
            .await
            .map_err(|e| internal_err(format!(
                "eth api block_by_number spawn_blocking get_transaction_status_by_block_hash error: {:?}", e.to_string()
            )))?;

            let transaction_statuses = match transaction_statuses_result {
                Ok(Some(value)) => value,
                Ok(None) => return Ok(None),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api block_by_number get_transaction_status_by_block_hash error: {:?}",
                        e.to_string()
                    )))
                }
            };

            Ok(Some(Self::rich_block_build(
                &block,
                transaction_statuses,
                full,
            )))
        })
    }

    fn transaction_count(
        &self,
        address: H160,
        number: Option<BlockNumber>,
    ) -> BoxFuture<Result<U256>> {
        log::info!(target: "eth api", "transaction_count address:{:?} number:{:?}", &address, &number);

        let getter = self.getter.clone();

        Box::pin(async move {
            if let Some(BlockNumber::Pending) = number {
                let pending_nonce_result =
                    tokio::task::spawn_blocking(move || getter.get_pending_nonce(address))
                        .await
                        .map_err(|e| {
                            internal_err(format!(
                        "eth api transaction_count spawn_blocking get_pending_nonce error: {:?}",
                        e.to_string()
                    ))
                        })?;

                let nonce = match pending_nonce_result {
                    Ok(Some(val)) => val,
                    Ok(None) => return Ok(U256::zero()),
                    Err(e) => {
                        return Err(internal_err(format!(
                            "eth api transaction_count get_pending_nonce error: {:?}",
                            e.to_string()
                        )))
                    }
                };

                return Ok(nonce);
            }

            let height = match block_number_to_height(number, getter.clone()) {
                Ok(h) => h,
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api transaction_count block_number_to_height error: {:?}",
                        e.to_string()
                    )))
                }
            };

            let nonce_result =
                tokio::task::spawn_blocking(move || getter.get_nonce(height, address))
                    .await
                    .map_err(|e| {
                        internal_err(format!(
                            "eth api transaction_count spawn_blocking get_nonce error: {:?}",
                            e.to_string()
                        ))
                    })?;

            match nonce_result {
                Ok(nonce) => Ok(nonce),
                Err(e) => Err(internal_err(format!(
                    "eth api transaction_count get_nonce error: {:?}",
                    e.to_string()
                ))),
            }
        })
    }

    fn block_transaction_count_by_hash(&self, hash: H256) -> BoxFuture<Result<Option<U256>>> {
        log::info!(target: "eth api", "block_transaction_count_by_hash hash:{:?}", &hash);
        let getter = self.getter.clone();

        Box::pin(async move {
            let block = tokio::task::spawn_blocking(move || getter.get_block_by_hash(hash))
            .await
            .map_err(|e| internal_err(format!(
                "eth api block_transaction_count_by_hash spawn_blocking get_block_by_hash error:{:?}",
                e.to_string()
            )))?
            .map_err(|e| internal_err(format!(
                "eth api block_transaction_count_by_hash get_block_by_hash error:{:?}",
                e.to_string()
            )))?;

            Ok(Some(U256::from(
                block.map_or(0, |b| b.transactions.len() as u64),
            )))
        })
    }

    fn block_transaction_count_by_number(
        &self,
        number: BlockNumber,
    ) -> BoxFuture<Result<Option<U256>>> {
        log::info!(target: "eth api", "block_transaction_count_by_number number:{:?}", &number);
        let getter = self.getter.clone();

        Box::pin(async move {
            let getter_clone = getter.clone();
            let height_result = tokio::task::spawn_blocking(move || {
                block_number_to_height(Some(number), getter_clone.clone()).map_err(|e| {
                    internal_err(format!(
                    "eth api block_transaction_count_by_number block_number_to_height error:{:?}",
                    e.to_string()
                ))
                })
            })
            .await;

            let height = match height_result {
                Ok(Ok(h)) => h,
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(internal_err(format!(
                    "eth api block_transaction_count_by_number spawn_blocking block_number_to_height error:{:?}",
                    e.to_string()
                ))),
            };

            let getter_clone = getter.clone();
            let hash_result = tokio::task::spawn_blocking(move || {
                getter_clone
                    .get_block_hash_by_height(U256::from(height))
                    .map_err(|e| {
                        internal_err(format!(
                    "eth api block_transaction_count_by_number get_block_hash_by_height error:{:?}",
                    e.to_string()
                ))
                    })
            })
            .await;

            let hash = match hash_result {
                Ok(Ok(h)) => h,
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(internal_err(format!(
                    "eth api block_transaction_count_by_number spawn_blocking get_block_hash_by_height error:{:?}",
                    e.to_string()
                ))),
            };

            let hash = match hash {
                Some(h) => h,
                None => return Ok(None),
            };

            let block_result =
                tokio::task::spawn_blocking(move || getter.get_block_by_hash(hash)).await;

            let block = match block_result {
                Ok(Ok(b)) => b,
                Ok(Err(e)) => return Err(internal_err(format!(
                    "eth api block_transaction_count_by_number spawn_blocking get_block_by_hash error:{:?}",
                    e.to_string()
                ))),
                Err(e) => return Err(internal_err(format!(
                    "eth api block_transaction_count_by_number spawn_blocking get_block_by_hash error:{:?}",
                    e.to_string()
                ))),
            };

            match block {
                Some(b) => Ok(Some(U256::from(b.transactions.len()))),
                None => Ok(None),
            }
        })
    }

    fn block_uncles_count_by_hash(&self, _: H256) -> Result<U256> {
        log::info!(target: "eth api", "block_uncles_count_by_hash");
        Ok(U256::zero())
    }

    fn block_uncles_count_by_number(&self, _: BlockNumber) -> Result<U256> {
        log::info!(target: "eth api", "block_uncles_count_by_number");
        Ok(U256::zero())
    }

    fn code_at(&self, address: H160, number: Option<BlockNumber>) -> BoxFuture<Result<Bytes>> {
        log::info!(target: "eth api", "code_at address:{:?} number:{:?}", &address, &number);
        // FRA (FRC20 precompile)
        if address == H160::from_low_u64_be(0x1000) {
            return Box::pin(future::ok(Bytes::new(b"fra".to_vec())));
        }

        let getter = self.getter.clone();
        Box::pin(async move {
            let getter_clone = getter.clone();
            if let Some(BlockNumber::Pending) = number {
                let byte_code_result = tokio::task::spawn_blocking(move || {
                    getter_clone.get_pending_byte_code(address).map_err(|e| {
                        internal_err(format!(
                            "eth api code_at get_pending_byte_code error:{:?}",
                            e.to_string()
                        ))
                    })
                })
                .await;

                let byte_code = match byte_code_result {
                    Ok(Ok(code)) => code,
                    Ok(Err(e)) => return Err(e),
                    Err(e) => {
                        return Err(internal_err(format!(
                            "eth api code_at spawn_blocking get_pending_byte_code error:{:?}",
                            e.to_string()
                        )))
                    }
                };

                if let Some(code) = byte_code {
                    return Ok(code.into());
                } else {
                    return Ok(Bytes::new(vec![]));
                }
            };

            let getter_clone = getter.clone();
            let height_result = tokio::task::spawn_blocking(move || {
                block_number_to_height(number, getter_clone).map_err(|e| {
                    internal_err(format!(
                        "eth api code_at block_number_to_height error:{:?}",
                        e.to_string()
                    ))
                })
            })
            .await;

            let height = match height_result {
                Ok(Ok(h)) => h,
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api code_at spawn_blocking block_number_to_height error:{:?}",
                        e.to_string()
                    )))
                }
            };

            let code_result = tokio::task::spawn_blocking(move || {
                getter.get_byte_code(height, address).map_err(|e| {
                    internal_err(format!(
                        "eth api code_at get_byte_code error:{:?}",
                        e.to_string()
                    ))
                })
            })
            .await;

            match code_result {
                Ok(Ok(code)) => Ok(code.into()),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(internal_err(format!(
                    "eth api code_at spawn_blocking get_byte_code error:{:?}",
                    e.to_string()
                ))),
            }
        })
    }

    fn send_raw_transaction(&self, bytes: Bytes) -> BoxFuture<Result<H256>> {
        let transaction = match rlp::decode::<LegacyTransaction>(&bytes.0[..]) {
            Ok(transaction) => transaction,
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api send_raw_transaction decode tx error:{:?}",
                    e.to_string()
                ))));
            }
        };
        log::info!(target: "eth api", "send_raw_transaction bytes:{:?}", &transaction);

        let transaction_hash =
            H256::from_slice(Keccak256::digest(&rlp::encode(&transaction)).as_slice());

        let txn =
            serde_json::to_vec(&UncheckedTransaction::new_unsigned(transaction)).map_err(|e| {
                internal_err(format!(
                    "eth api send_raw_transaction serde_json::to_vec tx error:{:?}",
                    e.to_string()
                ))
            });

        if let Err(e) = txn {
            return Box::pin(future::err(e));
        }

        // check_tx and broadcast
        let client = self.tm_client.clone();
        let txn_with_tag = EvmRawTxWrapper::wrap(&txn.unwrap());
        let (tx, rx) = mpsc::channel();
        RT.spawn(async move {
            let resp = client.broadcast_tx_sync(txn_with_tag.into()).await;
            tx.send(resp).unwrap();
        });

        // fetch response
        if let Ok(resp) = rx.recv().unwrap() {
            if resp.code != Code::Ok {
                return Box::pin(future::err(internal_err(format!(
                    "eth api send_raw_transaction rx.recv() error:{:?}",
                    resp.log
                ))));
            }
        } else {
            return Box::pin(future::err(internal_err(String::from(
                "eth api send_raw_transaction: broadcast_tx_sync failed",
            ))));
        }
        Box::pin(future::ok(transaction_hash))
    }

    fn estimate_gas(
        &self,
        request: CallRequest,
        number: Option<BlockNumber>,
    ) -> BoxFuture<Result<U256>> {
        log::info!(target: "eth api", "estimate_gas request:{:?} number:{:?}", &request, &number);
        let is_pending = matches!(number, Some(BlockNumber::Pending));
        let getter = self.getter.clone();
        let gas_price = self.gas_price;
        let chain_id = self.chain_id;
        let tendermint_url = self.tendermint_url.clone();

        Box::pin(async move {
            let getter_clone = getter.clone();
            let height_result = tokio::task::spawn_blocking(move || {
                block_number_to_height(number, getter_clone).map_err(|e| {
                    internal_err(format!(
                        "eth api estimate_gas block_number_to_height error:{:?}",
                        e.to_string()
                    ))
                })
            })
            .await;

            let height = match height_result {
                Ok(Ok(h)) => h,
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api estimate_gas spawn_blocking block_number_to_height error:{:?}",
                        e.to_string()
                    )))
                }
            };

            let getter_clone = getter.clone();
            let block_result = tokio::task::spawn_blocking(move || {
                getter_clone
                    .get_block_hash_by_height(U256::from(height))
                    .map_err(|e| {
                        internal_err(format!(
                            "eth api estimate_gas get_block_hash_by_height error:{:?}",
                            e.to_string()
                        ))
                    })
            })
            .await;

            let hash = match block_result {
                Ok(Ok(hash)) => hash,
                Ok(Ok(None)) => None,
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api estimate_gas spawn_blocking get_block_hash_by_height error:{:?}",
                        e.to_string()
                    )))
                }
            };

            let block_result = match hash {
                Some(h) => {
                    let getter_clone = getter.clone();
                    tokio::task::spawn_blocking(move || {
                        getter_clone.get_block_by_hash(h).map_err(|e| {
                            internal_err(format!(
                                "eth api estimate_gas get_block_by_hash error:{:?}",
                                e.to_string()
                            ))
                        })
                    })
                    .await
                }
                None => Ok(Ok(None)),
            };

            let block = match block_result {
                Ok(Ok(b)) => b,
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(internal_err(format!(
                        "eth api estimate_gas spawn_blocking get_block_by_hash error:{:?}",
                        e.to_string()
                    )))
                }
            };

            let mut highest = if let Some(gas) = request.gas {
                gas
            } else if let Some(b) = block {
                b.header.gas_limit
            } else {
                U256::from(u32::MAX)
            };

            if let Some(from) = request.from {
                let gas_price = request.gas_price.unwrap_or_default();
                if gas_price > U256::zero() {
                    let getter_clone = getter.clone();
                    let balance_result = tokio::task::spawn_blocking(move || {
                        getter_clone.get_balance(height, from).map_err(|e| {
                            internal_err(format!(
                                "eth api estimate_gas get_balance error:{:?}",
                                e.to_string()
                            ))
                        })
                    })
                    .await;

                    let balance = match balance_result {
                        Ok(Ok(b)) => b,
                        Ok(Err(e)) => return Err(e),
                        Err(e) => {
                            return Err(internal_err(format!(
                                "eth api estimate_gas spawn_blocking get_balance error:{:?}",
                                e.to_string()
                            )))
                        }
                    };

                    let mut available = balance;
                    if let Some(value) = request.value {
                        if value > available {
                            return Err(internal_err(
                                "eth api estimate_gas insufficient funds for transfer",
                            ));
                        }
                        available -= value;
                    }
                    let allowance = available / gas_price;
                    if highest < allowance {
                        log::warn!(
                        "Gas estimation capped by limited funds original {} balance {} sent {} feecap {} fundable {}",
                        highest,
                        balance,
                        request.value.unwrap_or_default(),
                        gas_price,
                        allowance
                    );
                        highest = allowance;
                    }
                }
            }

            let execute_call_or_create =
                |request: CallRequest, gas_limit: u64| -> (Vec<u8>, ExitReason, U256) {
                    let data = request.data.map(|d| d.0).unwrap_or_default();
                    let config = evm::Config::istanbul();
                    let metadata = StackSubstateMetadata::new(gas_limit, &config);
                    let precompile_set = Web3EvmPrecompiles::new(height);

                    let mut executor = StackExecutor::new_with_precompiles(
                        Web3EvmStackstate::new(
                            U256::from(gas_price),
                            chain_id,
                            height,
                            is_pending,
                            request.from.unwrap_or_default(),
                            getter.clone(),
                            tendermint_url.as_str(),
                            metadata,
                        ),
                        &config,
                        &precompile_set,
                    );
                    let access_list = Vec::new();

                    if let Some(to) = request.to {
                        let (exit_reason, data) = executor.transact_call(
                            request.from.unwrap_or_default(),
                            to,
                            request.value.unwrap_or_default(),
                            data,
                            gas_limit,
                            access_list,
                        );
                        (data, exit_reason, U256::from(executor.used_gas()))
                    } else {
                        let (exit_reason, data) = executor.transact_create(
                            request.from.unwrap_or_default(),
                            request.value.unwrap_or_default(),
                            data,
                            gas_limit,
                            access_list,
                        );
                        (data, exit_reason, U256::from(executor.used_gas()))
                    }
                };

            let (data, exit_reason, used_gas) =
                execute_call_or_create(request.clone(), highest.low_u64());

            if let Err(e) = Self::error_on_execution_failure(&exit_reason, &data) {
                return Err(e);
            }

            let mut lowest = U256::from(21_000);
            let mut mid = std::cmp::min(used_gas * 3, (highest + lowest) / 2);
            let mut previous_highest = highest;

            while (highest - lowest) > U256::one() {
                let (data, exit_reason, _) = execute_call_or_create(request.clone(), mid.low_u64());
                match exit_reason {
                    ExitReason::Succeed(_) => {
                        highest = mid;
                        if (previous_highest - highest) * 10 / previous_highest < U256::one() {
                            return Ok(highest);
                        }
                        previous_highest = highest;
                    }
                    ExitReason::Revert(_) | ExitReason::Error(ExitError::OutOfGas) => {
                        lowest = mid;
                    }
                    other => {
                        if let Err(e) = Self::error_on_execution_failure(&other, &data) {
                            return Err(e);
                        }
                    }
                }
                mid = (highest + lowest) / 2;
            }

            Ok(used_gas)
        })
    }

    fn transaction_by_hash(&self, tx_hash: H256) -> BoxFuture<Result<Option<Transaction>>> {
        log::info!(target: "eth api", "transaction_by_hash tx_hash:{:?}", &tx_hash);
        let getter = self.getter.clone();

        Box::pin(async move {
            let getter_clone = getter.clone();
            let index_result = tokio::task::spawn_blocking(move || {
            getter_clone .get_transaction_index_by_tx_hash(tx_hash)
                .map_err(|e| internal_err(format!(
                    "eth api transaction_by_hash get_transaction_index_by_tx_hash error:{:?}",
                    e.to_string()
                )))
        })
        .await
        .map_err(|e| internal_err(format!(
            "eth api transaction_by_hash spawn_blocking get_transaction_index_by_tx_hash error:{:?}",
            e.to_string()
        )))?;

            let (hash, index) = match index_result {
                Ok(Some(value)) => value,
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            };

            let getter_clone = getter.clone();
            let block_result = tokio::task::spawn_blocking(move || {
                getter_clone.get_block_by_hash(hash).map_err(|e| {
                    internal_err(format!(
                        "eth api transaction_by_hash get_block_by_hash error:{:?}",
                        e.to_string()
                    ))
                })
            })
            .await
            .map_err(|e| {
                internal_err(format!(
                    "eth api transaction_by_hash spawn_blocking get_block_by_hash error:{:?}",
                    e.to_string()
                ))
            })?;

            let block = match block_result {
                Ok(Some(b)) => b,
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            };

            let statuses_result = tokio::task::spawn_blocking(move || {
            getter.get_transaction_status_by_block_hash(hash)
                .map_err(|e| internal_err(format!(
                    "eth api transaction_by_hash get_transaction_status_by_block_hash error:{:?}",
                    e.to_string()
                )))
        })
        .await
        .map_err(|e| internal_err(format!(
            "eth api transaction_by_hash spawn_blocking get_transaction_status_by_block_hash error:{:?}",
            e.to_string()
        )))?;

            let transaction_statuses = match statuses_result {
                Ok(Some(statuses)) => statuses,
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            };

            let transaction = match block.transactions.get(index as usize) {
                Some(tx) => tx,
                None => return Ok(None),
            };

            let transaction_status = match transaction_statuses.get(index as usize) {
                Some(status) => status,
                None => return Ok(None),
            };

            Ok(Some(Self::transaction_build(
                &block,
                transaction,
                transaction_status,
            )))
        })
    }

    fn transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> BoxFuture<Result<Option<Transaction>>> {
        log::info!(target: "eth api", "transaction_by_block_hash_and_index hash:{:?} index:{:?}", &hash, &index);
        let block = match self.getter.get_block_by_hash(hash) {
            Ok(value) => {
                if let Some(hash_index) = value {
                    hash_index
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_by_block_hash_and_index get_block_by_hash error:{:?}",
                    e.to_string()
                ))));
            }
        };
        let transaction = if let Some(hash_index) = block.transactions.get(index.value()) {
            hash_index
        } else {
            return Box::pin(future::ok(None));
        };
        let transaction_statuses = match self.getter.get_transaction_status_by_block_hash(hash) {
            Ok(value) => {
                if let Some(statuses) = value {
                    statuses
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_by_block_hash_and_index get_transaction_status_by_block_hash error:{:?}",
                e.to_string()
                ))));
            }
        };
        let transaction_status = if let Some(status) = transaction_statuses.get(index.value()) {
            status
        } else {
            return Box::pin(future::ok(None));
        };
        Box::pin(future::ok(Some(Self::transaction_build(
            &block,
            transaction,
            transaction_status,
        ))))
    }

    fn transaction_by_block_number_and_index(
        &self,
        number: BlockNumber,
        index: Index,
    ) -> BoxFuture<Result<Option<Transaction>>> {
        log::info!(target: "eth api", "transaction_by_block_number_and_index number:{:?} index:{:?}", &number, &index);
        let height = match block_number_to_height(Some(number), self.getter.clone()) {
            Ok(h) => h,
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_by_block_number_and_index block_number_to_height error:{:?}",
                e.to_string()
                ))));
            }
        };
        let hash = match self.getter.get_block_hash_by_height(U256::from(height)) {
            Ok(value) => {
                if let Some(hash) = value {
                    hash
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_by_block_number_and_index get_block_hash_by_height error:{:?}",
                e.to_string()
                ))));
            }
        };

        let block = match self.getter.get_block_by_hash(hash) {
            Ok(value) => {
                if let Some(hash_index) = value {
                    hash_index
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_by_block_number_and_index get_block_by_hash error:{:?}",
                    e.to_string()
                ))));
            }
        };
        let transaction = if let Some(hash_index) = block.transactions.get(index.value()) {
            hash_index
        } else {
            return Box::pin(future::ok(None));
        };
        let transaction_statuses = match self.getter.get_transaction_status_by_block_hash(hash) {
            Ok(value) => {
                if let Some(statuses) = value {
                    statuses
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_by_block_number_and_index get_transaction_status_by_block_hash error:{:?}",
                e.to_string()
                ))));
            }
        };
        let transaction_status = if let Some(status) = transaction_statuses.get(index.value()) {
            status
        } else {
            return Box::pin(future::ok(None));
        };
        Box::pin(future::ok(Some(Self::transaction_build(
            &block,
            transaction,
            transaction_status,
        ))))
    }

    fn transaction_receipt(&self, tx_hash: H256) -> BoxFuture<Result<Option<Receipt>>> {
        log::info!(target: "eth api", "transaction_receipt tx_hash:{:?}", &tx_hash);
        let (hash, index) = match self.getter.get_transaction_index_by_tx_hash(tx_hash) {
            Ok(value) => {
                if let Some(hash_index) = value {
                    hash_index
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_receipt get_block_by_hash error:{:?}",
                    e.to_string()
                ))));
            }
        };
        let block = match self.getter.get_block_by_hash(hash) {
            Ok(value) => {
                if let Some(hash_index) = value {
                    hash_index
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_receipt get_block_by_hash error:{:?}",
                    e.to_string()
                ))));
            }
        };
        let statuses = match self.getter.get_transaction_status_by_block_hash(hash) {
            Ok(value) => {
                if let Some(hash_index) = value {
                    hash_index
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_receipt get_transaction_status_by_block_hash error:{:?}",
                    e.to_string()
                ))));
            }
        };
        let status = statuses[index as usize].clone();

        let receipts = match self.getter.get_transaction_receipt_by_block_hash(hash) {
            Ok(value) => {
                if let Some(hash_index) = value {
                    hash_index
                } else {
                    return Box::pin(future::ok(None));
                }
            }
            Err(e) => {
                return Box::pin(future::err(internal_err(format!(
                    "eth api transaction_receipt get_transaction_receipt_by_block_hash error:{:?}",
                    e.to_string()
                ))));
            }
        };
        let receipt = receipts[index as usize].clone();
        let (logs, status_code, logs_bloom, used_gas) = match receipt {
            ReceiptAny::Frontier(r) => (
                r.logs,
                U64::from(r.state_root.to_low_u64_be()),
                r.logs_bloom,
                r.used_gas,
            ),
            ReceiptAny::EIP658(r) => (r.logs, U64::from(r.status_code), r.logs_bloom, r.used_gas),
            ReceiptAny::EIP2930(r) => (r.logs, U64::from(r.status_code), r.logs_bloom, r.used_gas),
            ReceiptAny::EIP1559(r) => (r.logs, U64::from(r.status_code), r.logs_bloom, r.used_gas),
        };
        let mut cumulative_receipts = receipts;
        cumulative_receipts.truncate((status.transaction_index + 1) as usize);

        let receipt = Receipt {
            transaction_hash: Some(tx_hash),
            transaction_index: Some(index.into()),
            block_hash: Some(block.header.hash()),
            from: Some(status.from),
            to: status.to,
            block_number: Some(block.header.number),
            cumulative_gas_used: {
                let cumulative_gas: u32 = cumulative_receipts
                    .iter()
                    .map(|r| {
                        match r {
                            ReceiptAny::Frontier(r) => r.used_gas,
                            ReceiptAny::EIP658(r) => r.used_gas,
                            ReceiptAny::EIP2930(r) => r.used_gas,
                            ReceiptAny::EIP1559(r) => r.used_gas,
                        }
                        .as_u32()
                    })
                    .sum();
                U256::from(cumulative_gas)
            },
            gas_used: Some(used_gas),
            contract_address: status.contract_address,
            logs: {
                let mut pre_receipts_log_index = None;
                if !cumulative_receipts.is_empty() {
                    cumulative_receipts.truncate(cumulative_receipts.len() - 1);
                    pre_receipts_log_index = Some(
                        cumulative_receipts
                            .iter()
                            .map(|_| logs.len() as u32)
                            .sum::<u32>(),
                    );
                }
                logs.iter()
                    .enumerate()
                    .map(|(i, log)| Log {
                        address: log.address,
                        topics: log.topics.clone(),
                        data: Bytes(log.data.clone()),
                        block_hash: Some(block.header.hash()),
                        block_number: Some(block.header.number),
                        transaction_hash: Some(status.transaction_hash),
                        transaction_index: Some(status.transaction_index.into()),
                        log_index: Some(U256::from(
                            (pre_receipts_log_index.unwrap_or(0)) + i as u32,
                        )),
                        transaction_log_index: Some(U256::from(i)),
                        removed: false,
                    })
                    .collect()
            },
            status_code: Some(status_code),
            logs_bloom,
            state_root: None,
        };

        Box::pin(future::ok(Some(receipt)))
    }

    fn uncle_by_block_hash_and_index(&self, _: H256, _: Index) -> Result<Option<RichBlock>> {
        log::info!(target: "eth api", "uncle_by_block_hash_and_index");
        Ok(None)
    }

    fn uncle_by_block_number_and_index(
        &self,
        _: BlockNumber,
        _: Index,
    ) -> Result<Option<RichBlock>> {
        log::info!(target: "eth api", "uncle_by_block_number_and_index");
        Ok(None)
    }

    fn logs(&self, filter: Filter) -> BoxFuture<Result<Vec<Log>>> {
        log::info!(target: "eth api", "logs filter:{:?}", &filter);
        let mut ret: Vec<Log> = Vec::new();
        if let Some(block_hash) = filter.block_hash {
            let block = match self.getter.get_block_by_hash(block_hash) {
                Ok(value) => {
                    if let Some(b) = value {
                        b
                    } else {
                        return Box::pin(future::err(internal_err(String::new())));
                    }
                }
                Err(e) => {
                    return Box::pin(future::err(internal_err(format!(
                        "eth api logs get_block_by_hash error:{:?}",
                        e.to_string()
                    ))));
                }
            };

            match self.getter.get_transaction_status_by_block_hash(block_hash) {
                Ok(value) => {
                    if let Some(statuses) = value {
                        filter_block_logs(&mut ret, &filter, block, statuses);
                    }
                }
                Err(e) => {
                    return Box::pin(future::err(internal_err(format!(
                        "eth api logs get_transaction_status_by_block_hash error:{:?}",
                        e.to_string()
                    ))));
                }
            };
        } else {
            let current_number = match self.getter.latest_height() {
                Ok(height) => height,
                Err(e) => {
                    return Box::pin(future::err(internal_err(format!(
                        "eth api logs latest_height error:{:?}",
                        e.to_string()
                    ))));
                }
            };

            let from_number = filter
                .from_block
                .clone()
                .and_then(|v| v.to_min_block_num())
                .map(|from| {
                    if from as u32 > current_number {
                        current_number
                    } else {
                        from as u32
                    }
                })
                .unwrap_or(current_number);

            let to_number = filter
                .to_block
                .clone()
                .and_then(|v| v.to_min_block_num())
                .map(|to| {
                    if to as u32 > current_number {
                        current_number
                    } else {
                        to as u32
                    }
                })
                .unwrap_or(current_number);

            let topics_input = if filter.topics.is_some() {
                let filtered_params = FilteredParams::new(Some(filter.clone()));
                Some(filtered_params.flat_topics)
            } else {
                None
            };
            let address_bloom_filter = FilteredParams::addresses_bloom_filter(&filter.address);
            let topics_bloom_filter = FilteredParams::topics_bloom_filter(&topics_input);

            let mut current = to_number;
            while current >= from_number {
                let block_hash = match self.getter.get_block_hash_by_height(U256::from(current)) {
                    Ok(value) => {
                        if let Some(hash) = value {
                            hash
                        } else {
                            return Box::pin(future::err(internal_err(
                                "eth api logs get_block_hash_by_height return none",
                            )));
                        }
                    }
                    Err(e) => {
                        return Box::pin(future::err(internal_err(format!(
                            "eth api logs get_block_hash_by_height error:{:?}",
                            e.to_string()
                        ))));
                    }
                };

                let block = match self.getter.get_block_by_hash(block_hash) {
                    Ok(value) => {
                        if let Some(b) = value {
                            b
                        } else {
                            return Box::pin(future::err(internal_err(
                                "eth api logs get_block_by_hash return none",
                            )));
                        }
                    }
                    Err(e) => {
                        return Box::pin(future::err(internal_err(format!(
                            "eth api logs get_block_by_hash error:{:?}",
                            e.to_string()
                        ))));
                    }
                };

                if FilteredParams::address_in_bloom(block.header.logs_bloom, &address_bloom_filter)
                    && FilteredParams::topics_in_bloom(
                        block.header.logs_bloom,
                        &topics_bloom_filter,
                    )
                {
                    match self.getter.get_transaction_status_by_block_hash(block_hash) {
                        Ok(value) => {
                            if let Some(statuses) = value {
                                let mut logs: Vec<Log> = Vec::new();
                                filter_block_logs(&mut logs, &filter, block, statuses);
                                ret.append(&mut logs);
                            }
                        }
                        Err(e) => {
                            return Box::pin(future::err(internal_err(format!(
                                "eth api logs get_transaction_status_by_block_hash error:{:?}",
                                e.to_string()
                            ))));
                        }
                    };
                }

                // Check for restrictions
                if ret.len() as u32 > MAX_PAST_LOGS {
                    break;
                }
                if 0 == current {
                    break;
                } else {
                    current -= 1;
                }
            }
        }
        Box::pin(future::ok(ret))
    }

    fn work(&self) -> Result<Work> {
        log::info!(target: "eth api", "work");
        Ok(Work {
            pow_hash: H256::default(),
            seed_hash: H256::default(),
            target: H256::default(),
            number: None,
        })
    }

    fn submit_work(&self, _: H64, _: H256, _: H256) -> Result<bool> {
        log::info!(target: "eth api", "submit_work");
        Ok(false)
    }

    fn submit_hashrate(&self, _: U256, _: H256) -> Result<bool> {
        log::info!(target: "eth api", "submit_hashrate");
        Ok(false)
    }
}
pub fn filter_block_logs<'a>(
    ret: &'a mut Vec<Log>,
    filter: &'a Filter,
    block: evm_exporter::Block,
    transaction_statuses: Vec<TransactionStatus>,
) -> &'a Vec<Log> {
    let params = FilteredParams::new(Some(filter.clone()));
    let mut block_log_index: u32 = 0;
    let block_hash = H256::from_slice(Keccak256::digest(&rlp::encode(&block.header)).as_slice());
    for status in transaction_statuses.iter() {
        let logs = status.logs.clone();
        let transaction_hash = status.transaction_hash;
        for (transaction_log_index, ethereum_log) in logs.into_iter().enumerate() {
            let mut log = Log {
                address: ethereum_log.address,
                topics: ethereum_log.topics.clone(),
                data: Bytes(ethereum_log.data.clone()),
                block_hash: None,
                block_number: None,
                transaction_hash: None,
                transaction_index: None,
                log_index: None,
                transaction_log_index: None,
                removed: false,
            };
            let mut add: bool = true;
            match (filter.address.clone(), filter.topics.clone()) {
                (Some(_), Some(_)) => {
                    if !params.filter_address(&log) || !params.filter_topics(&log) {
                        add = false;
                    }
                }
                (Some(_), None) => {
                    if !params.filter_address(&log) {
                        add = false;
                    }
                }
                (None, Some(_)) => {
                    if !params.filter_topics(&log) {
                        add = false;
                    }
                }
                (None, None) => {}
            }
            if add {
                log.block_hash = Some(block_hash);
                log.block_number = Some(block.header.number);
                log.transaction_hash = Some(transaction_hash);
                log.transaction_index = Some(U256::from(status.transaction_index));
                log.log_index = Some(U256::from(block_log_index));
                log.transaction_log_index = Some(U256::from(transaction_log_index));
                ret.push(log);
            }
            block_log_index += 1;
        }
    }
    ret
}
