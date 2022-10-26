use {
    super::{eth::filter_block_logs, internal_err, MAX_PAST_LOGS, MAX_STORED_FILTERS},
    ethereum_types::{H256, U256},
    evm_exporter::{Block, Getter, TransactionStatus, PREFIX},
    futures::executor::ThreadPool,
    jsonrpc_core::Result,
    lazy_static::lazy_static,
    std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
        thread, time,
    },
    web3_rpc_core::{
        types::{
            BlockNumber, Filter, FilterChanges, FilterPool, FilterPoolItem, FilterType,
            FilteredParams, Index, Log,
        },
        EthFilterApi,
    },
};

lazy_static! {
    static ref POOL_FILTER: ThreadPool =
        ThreadPool::new().expect("Failed to create EthFilter thread pool executor");
}
const MAX_FILTER_SECS: u64 = 10;
const FILTER_RETAIN_THRESHOLD: u64 = 100;

pub struct EthFilterApiImpl {
    filter_pool: FilterPool,
    redis_pool: Arc<r2d2::Pool<redis::Client>>,
}

impl EthFilterApiImpl {
    pub fn new(redis_pool: Arc<r2d2::Pool<redis::Client>>) -> Self {
        let pool = Arc::new(Mutex::new(BTreeMap::new()));
        let instance = Self {
            filter_pool: pool.clone(),
            redis_pool: redis_pool.clone(),
        };
        POOL_FILTER.spawn_ok(Self::filter_pool_task(redis_pool, pool));
        instance
    }
    fn block_number(&self) -> Result<u64> {
        let mut conn = self
            .redis_pool
            .get()
            .map_err(|e| internal_err(e.to_string()))?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        getter
            .latest_height()
            .map_err(|e| internal_err(e.to_string()))
            .map(|val| val as u64)
    }
    fn create_filter(&self, filter_type: FilterType) -> Result<U256> {
        let block_number = self.block_number()?;

        let pool = self.filter_pool.clone();
        let response = if let Ok(locked) = &mut pool.lock() {
            if locked.len() >= MAX_STORED_FILTERS {
                return Err(internal_err(format!(
                    "Filter pool is full (limit {:?}).",
                    MAX_STORED_FILTERS
                )));
            }
            let last_key = match locked.iter().next_back() {
                Some((k, _)) => *k,
                None => U256::zero(),
            };
            // Assume `max_stored_filters` is always < U256::max.
            let key = last_key.checked_add(U256::one()).unwrap();
            locked.insert(
                key,
                FilterPoolItem {
                    last_poll: BlockNumber::Num(block_number),
                    filter_type,
                    at_block: block_number,
                },
            );
            Ok(key)
        } else {
            Err(internal_err("Filter pool is not available."))
        };
        response
    }

    async fn filter_pool_task(
        redis_pool: Arc<r2d2::Pool<redis::Client>>,
        filter_pool: Arc<Mutex<BTreeMap<U256, FilterPoolItem>>>,
    ) {
        let mut conn = redis_pool.get().expect("get redis connect");
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let mut last_height = getter.latest_height().expect("redis latest_height error");
        let ten_millis = time::Duration::from_millis(100);
        loop {
            thread::sleep(ten_millis);
            let height = match getter.latest_height() {
                Ok(val) => val,
                Err(_) => continue,
            };
            if last_height == height {
                continue;
            }
            if let Ok(filter_pool) = &mut filter_pool.lock() {
                let remove_list: Vec<_> = filter_pool
                    .iter()
                    .filter_map(|(&k, v)| {
                        let lifespan_limit = v.at_block + FILTER_RETAIN_THRESHOLD;
                        if U256::from(lifespan_limit) <= U256::from(height) {
                            Some(k)
                        } else {
                            None
                        }
                    })
                    .collect();

                for key in remove_list {
                    filter_pool.remove(&key);
                }
                last_height = height;
            }
        }
    }

    fn get_block(&self, height: u64) -> Result<Option<Block>> {
        let mut conn = self
            .redis_pool
            .get()
            .map_err(|e| internal_err(e.to_string()))?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let hash = match getter
            .get_block_hash_by_height(U256::from(height))
            .map_err(|e| internal_err(e.to_string()))?
        {
            Some(hash) => hash,
            None => {
                return Ok(None);
            }
        };
        getter
            .get_block_by_hash(hash)
            .map_err(|e| internal_err(e.to_string()))
    }
    fn get_transaction_statuses(&self, height: u64) -> Result<Option<Vec<TransactionStatus>>> {
        let mut conn = self
            .redis_pool
            .get()
            .map_err(|e| internal_err(e.to_string()))?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let hash = match getter
            .get_block_hash_by_height(U256::from(height))
            .map_err(|e| internal_err(e.to_string()))?
        {
            Some(hash) => hash,
            None => {
                return Ok(None);
            }
        };
        getter
            .get_transaction_status_by_block_hash(hash)
            .map_err(|e| internal_err(e.to_string()))
    }

    fn filter_range_logs(
        &self,
        ret: &mut Vec<Log>,

        filter: &Filter,
        from: u64,
        to: u64,
    ) -> Result<()> {
        // Max request duration of 10 seconds.
        let max_duration = time::Duration::from_secs(MAX_FILTER_SECS);
        let begin_request = time::Instant::now();

        // Pre-calculate BloomInput for reuse.
        let topics_input = if filter.topics.is_some() {
            let filtered_params = FilteredParams::new(Some(filter.clone()));
            Some(filtered_params.flat_topics)
        } else {
            None
        };
        let address_bloom_filter = FilteredParams::addresses_bloom_filter(&filter.address);
        let topics_bloom_filter = FilteredParams::topics_bloom_filter(&topics_input);

        let mut current = to;
        while current >= from {
            let block = self.get_block(current)?;

            if let Some(block) = block {
                if FilteredParams::address_in_bloom(block.header.logs_bloom, &address_bloom_filter)
                    && FilteredParams::topics_in_bloom(
                        block.header.logs_bloom,
                        &topics_bloom_filter,
                    )
                {
                    let statuses = self.get_transaction_statuses(current)?;
                    if let Some(statuses) = statuses {
                        filter_block_logs(ret, filter, block, statuses);
                    }
                }
            } else {
                // stop when past the 1st ethereum block
                break;
            }

            // Check for restrictions
            if ret.len() as u32 > MAX_PAST_LOGS {
                return Err(internal_err(format!(
                    "query returned more than {} results",
                    MAX_PAST_LOGS
                )));
            }
            if begin_request.elapsed() > max_duration {
                return Err(internal_err(format!(
                    "query timeout of {} seconds exceeded",
                    max_duration.as_secs()
                )));
            }
            current = current.saturating_sub(1);
        }
        Ok(())
    }
}

impl EthFilterApi for EthFilterApiImpl {
    fn new_filter(&self, filter: Filter) -> Result<U256> {
        self.create_filter(FilterType::Log(filter))
    }

    fn new_block_filter(&self) -> Result<U256> {
        self.create_filter(FilterType::Block)
    }

    fn new_pending_transaction_filter(&self) -> Result<U256> {
        Err(internal_err(
            "new_pending_transaction_filter method not available.",
        ))
    }

    fn filter_changes(&self, index: Index) -> Result<FilterChanges> {
        let key = U256::from(index.value());
        let cur_number = self.block_number()?;
        let pool = self.filter_pool.clone();
        // Try to lock.
        let response = if let Ok(locked) = &mut pool.lock() {
            // Try to get key.
            if let Some(pool_item) = locked.clone().get(&key) {
                match &pool_item.filter_type {
                    // For each block created since last poll, get a vector of ethereum hashes.
                    FilterType::Block => {
                        let last = pool_item.last_poll.to_min_block_num().unwrap();
                        let next = cur_number + 1;
                        let mut ethereum_hashes: Vec<H256> = Vec::new();
                        for n in last..next {
                            let block = self.get_block(n)?;
                            if let Some(block) = block {
                                ethereum_hashes.push(block.header.hash())
                            }
                        }
                        // Update filter `last_poll`.
                        locked.insert(
                            key,
                            FilterPoolItem {
                                last_poll: BlockNumber::Num(next),
                                filter_type: pool_item.clone().filter_type,
                                at_block: pool_item.at_block,
                            },
                        );
                        Ok(FilterChanges::Hashes(ethereum_hashes))
                    }
                    // For each event since last poll, get a vector of ethereum logs.
                    FilterType::Log(filter) => {
                        // Either the filter-specific `to` block or current block.
                        let mut to_number = filter
                            .to_block
                            .clone()
                            .and_then(|v| v.to_min_block_num())
                            .unwrap_or(cur_number);

                        if to_number > cur_number {
                            to_number = cur_number;
                        }

                        // The from clause is the max(last_poll, filter_from).
                        let last_poll = pool_item.last_poll.to_min_block_num().unwrap();
                        let filter_from = filter
                            .from_block
                            .clone()
                            .and_then(|v| v.to_min_block_num())
                            .unwrap_or(last_poll);

                        let from_number = std::cmp::max(last_poll, filter_from);

                        // Build the response.
                        let mut ret: Vec<Log> = Vec::new();
                        self.filter_range_logs(&mut ret, filter, from_number, to_number)?;
                        // Update filter `last_poll`.
                        locked.insert(
                            key,
                            FilterPoolItem {
                                last_poll: BlockNumber::Num(cur_number + 1),
                                filter_type: pool_item.clone().filter_type,
                                at_block: pool_item.at_block,
                            },
                        );
                        Ok(FilterChanges::Logs(ret))
                    }
                    // Should never reach here.
                    _ => Err(internal_err("Method not available.")),
                }
            } else {
                Err(internal_err(format!("Filter id {:?} does not exist.", key)))
            }
        } else {
            Err(internal_err("Filter pool is not available."))
        };
        response
    }

    fn filter_logs(&self, index: Index) -> Result<Vec<Log>> {
        let key = U256::from(index.value());
        let pool = self.filter_pool.clone();
        // Try to lock.
        let response = if let Ok(locked) = &mut pool.lock() {
            // Try to get key.
            if let Some(pool_item) = locked.clone().get(&key) {
                match &pool_item.filter_type {
                    FilterType::Log(filter) => {
                        let cur_number = self.block_number()?;
                        let from_number = filter
                            .from_block
                            .clone()
                            .and_then(|v| v.to_min_block_num())
                            .unwrap_or(cur_number);
                        let mut to_number = filter
                            .to_block
                            .clone()
                            .and_then(|v| v.to_min_block_num())
                            .unwrap_or(cur_number);

                        if to_number > cur_number {
                            to_number = cur_number;
                        }

                        let mut ret: Vec<Log> = Vec::new();
                        self.filter_range_logs(&mut ret, filter, from_number, to_number)?;
                        Ok(ret)
                    }
                    _ => Err(internal_err(format!(
                        "Filter id {:?} is not a Log filter.",
                        key
                    ))),
                }
            } else {
                Err(internal_err(format!("Filter id {:?} does not exist.", key)))
            }
        } else {
            Err(internal_err("Filter pool is not available."))
        };
        response
    }

    fn uninstall_filter(&self, index: Index) -> Result<bool> {
        let key = U256::from(index.value());
        let pool = self.filter_pool.clone();
        // Try to lock.
        let response = if let Ok(locked) = &mut pool.lock() {
            if locked.remove(&key).is_some() {
                Ok(true)
            } else {
                Err(internal_err(format!("Filter id {:?} does not exist.", key)))
            }
        } else {
            Err(internal_err("Filter pool is not available."))
        };
        response
    }
}
