use {
    crate::notify::SubscriberNotify,
    ethereum_types::{H256, U256},
    evm_exporter::{Block, Getter, Receipt, PREFIX},
    futures::{
        executor::ThreadPool,
        task::{FutureObj, Spawn, SpawnError},
        FutureExt, SinkExt, StreamExt,
    },
    jsonrpc_pubsub::{manager::SubscriptionManager, typed::Subscriber, SubscriptionId},
    lazy_static::lazy_static,
    sha3::{Digest, Keccak256},
    std::{collections::BTreeMap, sync::Arc},
    web3_rpc_core::{
        types::{
            pubsub::{Kind, Metadata, Params, PubSubSyncStatus, Result as PubSubResult},
            Bytes, FilteredParams, Header, Log, Rich,
        },
        EthPubSubApi,
    },
};
lazy_static! {
    static ref EXECUTOR: ThreadPool =
        ThreadPool::new().expect("Failed to create thread pool executor");
}

pub struct SubscriptionTaskExecutor;

impl Spawn for SubscriptionTaskExecutor {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        EXECUTOR.spawn_ok(future);
        Ok(())
    }

    fn status(&self) -> Result<(), SpawnError> {
        Ok(())
    }
}
pub struct EthPubSubApiImpl {
    redis_pool: Arc<r2d2::Pool<redis::Client>>,
    subscriptions: SubscriptionManager,
    subscriber_notify: Arc<SubscriberNotify>,
}
impl EthPubSubApiImpl {
    pub fn new(
        redis_pool: Arc<r2d2::Pool<redis::Client>>,
        subscriber_notify: Arc<SubscriberNotify>,
    ) -> Self {
        Self {
            redis_pool,
            subscriptions: SubscriptionManager::new(Arc::new(SubscriptionTaskExecutor)),
            subscriber_notify,
        }
    }
}
impl EthPubSubApi for EthPubSubApiImpl {
    type Metadata = Metadata;

    fn subscribe(
        &self,
        _metadata: Self::Metadata,
        subscriber: Subscriber<PubSubResult>,
        kind: Kind,
        params: Option<Params>,
    ) {
        log::debug!(target: "eth_rpc", "new subscribe: {:?}", kind);
        let filtered_params = match params {
            Some(Params::Logs(filter)) => FilteredParams::new(Some(filter)),
            _ => FilteredParams::default(),
        };
        match kind {
            Kind::Logs => {
                let redis_pool = self.redis_pool.clone();
                self.subscriptions.add(subscriber, |sink| {
                    let stream = self
                        .subscriber_notify
                        .logs_event_notify
                        .notification_stream()
                        .filter_map(move |block_height| {
                            let info = redis_pool.get().map(|mut conn| {
                                let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
                                match getter.get_block_hash_by_height(block_height) {
                                    Ok(Some(hash)) => {
                                        let block = match getter.get_block_by_hash(hash) {
                                            Ok(Some(b)) => Some(b),
                                            _ => None,
                                        };
                                        let receipt = match getter.get_transaction_receipt_by_block_hash(hash) {
                                            Ok(Some(b)) => Some(b),
                                            _ => None,
                                        };
                                        (block,receipt)
                                    },
                                    _ => (None, None),
                                }
                            });
                            match info {
                                Ok((Some(block),Some(receipts))) => {
                                    futures::future::ready(Some((block, receipts)))
                                }
                                _ => futures::future::ready(None),
                            }

                        })
                        .flat_map(move |(block, receipts)| {
                            futures::stream::iter(logs(
                                block,
                                receipts,
                                &filtered_params,
                            ))
                        })
                        .map(|x| {
                            Ok::<Result<PubSubResult, jsonrpc_core::types::error::Error>, ()>(Ok(
                                PubSubResult::Log(Box::new(x)),
                            ))
                        });
                    stream
                        .forward(sink.sink_map_err(
                            |e| log::warn!(target: "eth_rpc", "Error sending notifications: {:?}", e),
                        ))
                        .map(|_| ())
                });
            }
            Kind::NewHeads => {
                let redis_pool = self.redis_pool.clone();
                self.subscriptions.add(subscriber, move |sink| {
                    let stream = self
                        .subscriber_notify
                        .new_heads_event_notify
                        .notification_stream()
                        .filter_map(move |block_height| {
                            let block = redis_pool.get().map(|mut conn|{
                                let mut getter = Getter::new(&mut *conn, PREFIX.to_string()) ;
                                match getter.get_block_hash_by_height(block_height) {
                                    Ok(Some(hash)) => {
                                        match getter.get_block_by_hash(hash) {
                                            Ok(Some(b)) => Some(b),
                                            _ => None
                                        }
                                    },
                                    _ => None
                                }
                            });
                            let rich = match block {
                                Ok(Some(block)) => {
                                     Some(block_build(block))
                                },
                                _ =>  None,
                            };
                            futures::future::ready(rich)
                        })
                        .map(|header| Ok::<_, ()>(Ok(PubSubResult::Header(Box::new(header)))));

                    stream
                        .forward(sink.sink_map_err(
                            |e| log::warn!(target: "eth_rpc", "Error sending notifications: {:?}", e),
                        ))
                        .map(|_| ())
                });
            }
            Kind::NewPendingTransactions => {
                self.subscriptions.add(subscriber, |sink| {
                    let stream = self
                        .subscriber_notify
                        .new_pending_tx_hash_event_notify
                        .notification_stream()
                        .filter_map(move |txhash| {
                            if H256::default() != txhash {
                                futures::future::ready(Some(txhash))
                            } else {
                                futures::future::ready(None)
                            }
                        })
                        .map(|tx_hash| Ok::<_, ()>(Ok(PubSubResult::TransactionHash(tx_hash))));
                    stream
                        .forward(sink.sink_map_err(
                            |e| log::warn!(target: "eth_rpc", "Error sending notifications: {:?}", e),
                        ))
                        .map(|_| ())
                });
            }
            Kind::Syncing => {
                self.subscriptions.add(subscriber, |sink| {
                    let stream = self
                        .subscriber_notify
                        .syncing_event_notify
                        .notification_stream()
                        .map(|status| {
                            Ok::<_, ()>(Ok(PubSubResult::SyncState(PubSubSyncStatus {
                                syncing: status,
                            })))
                        });
                    stream
                        .forward(sink.sink_map_err(
                            |e| log::warn!(target: "eth_rpc", "Error sending notifications: {:?}", e),
                        ))
                        .map(|_| ())
                });
            }
        }
    }

    fn unsubscribe(
        &self,
        _metadata: Option<Self::Metadata>,
        subscription_id: SubscriptionId,
    ) -> jsonrpc_core::Result<bool> {
        log::debug!(target: "eth_rpc", "unsubscribe id: {:?}", subscription_id);
        Ok(self.subscriptions.cancel(subscription_id))
    }
}

pub fn logs(block: Block, receipts: Vec<Receipt>, params: &FilteredParams) -> Vec<Log> {
    let block_hash = Some(H256::from_slice(
        Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
    ));
    let mut logs: Vec<Log> = vec![];
    let mut log_index: u32 = 0;
    for (receipt_index, receipt) in receipts.into_iter().enumerate() {
        let transaction_hash: Option<H256> = if !get_logs(receipt.clone()).is_empty() {
            Some(H256::from_slice(
                Keccak256::digest(&rlp::encode(&block.transactions[receipt_index as usize]))
                    .as_slice(),
            ))
        } else {
            None
        };
        for (transaction_log_index, log) in get_logs(receipt.clone()).into_iter().enumerate() {
            if add_log(block_hash.unwrap(), &log, &block, params) {
                logs.push(Log {
                    address: log.address,
                    topics: log.topics,
                    data: Bytes(log.data),
                    block_hash,
                    block_number: Some(block.header.number),
                    transaction_hash,
                    transaction_index: Some(U256::from(receipt_index)),
                    log_index: Some(U256::from(log_index)),
                    transaction_log_index: Some(U256::from(transaction_log_index)),
                    removed: false,
                });
            }
            log_index += 1;
        }
    }
    logs
}
fn get_logs(receipt: Receipt) -> Vec<ethereum::Log> {
    match receipt {
        ethereum::ReceiptAny::Frontier(r) => r.logs,
        ethereum::ReceiptAny::EIP658(r) => r.logs,
        ethereum::ReceiptAny::EIP2930(r) => r.logs,
        ethereum::ReceiptAny::EIP1559(r) => r.logs,
    }
}
fn add_log(
    block_hash: H256,
    ethereum_log: &ethereum::Log,
    block: &Block,
    params: &FilteredParams,
) -> bool {
    let log = Log {
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
    if params.filter.is_some() {
        let block_number = block.header.number.as_u64();
        if !params.filter_block_range(block_number)
            || !params.filter_block_hash(block_hash)
            || !params.filter_address(&log)
            || !params.filter_topics(&log)
        {
            return false;
        }
    }
    true
}

fn block_build(block: Block) -> Rich<Header> {
    Rich {
        inner: Header {
            hash: Some(H256::from_slice(
                Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
            )),
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
            logs_bloom: block.header.logs_bloom,
            timestamp: U256::from(block.header.timestamp),
            difficulty: block.header.difficulty,
            seal_fields: vec![
                Bytes(block.header.mix_hash.as_bytes().to_vec()),
                Bytes(block.header.nonce.as_bytes().to_vec()),
            ],
            size: Some(U256::from(rlp::encode(&block).len() as u32)),
        },
        extra_info: BTreeMap::new(),
    }
}
