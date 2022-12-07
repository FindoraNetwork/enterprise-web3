use {
    crate::notify::notifications::Notifications,
    ethereum_types::{H256, U256},
    evm_exporter::{Getter, PREFIX},
    ruc::*,
    serde_json::Value,
    sha2::{Digest, Sha256},
    std::{sync::Arc, thread, time},
};

#[cfg(feature = "cluster_redis")]
pub struct SubscriberNotify {
    client: Arc<redis::cluster::ClusterClient>,
    pub logs_event_notify: Arc<Notifications<U256>>,
    pub new_heads_event_notify: Arc<Notifications<U256>>,
    pub new_pending_tx_hash_event_notify: Arc<Notifications<H256>>,
    pub syncing_event_notify: Arc<Notifications<bool>>,
}

#[cfg(not(feature = "cluster_redis"))]
pub struct SubscriberNotify {
    tm_url: String,
    millis: u64,
    redis_pool: Arc<r2d2::Pool<redis::Client>>,
    pub logs_event_notify: Arc<Notifications<U256>>,
    pub new_heads_event_notify: Arc<Notifications<U256>>,
    pub new_pending_tx_hash_event_notify: Arc<Notifications<H256>>,
    pub syncing_event_notify: Arc<Notifications<bool>>,
}

impl SubscriberNotify {
    #[cfg(feature = "cluster_redis")]
    pub fn new(client: Arc<redis::cluster::ClusterClient>, tm_url: &str) -> Self {
        Self {
            tm_url: String::from(tm_url),
            millis: 2000,
            client,
            tm_client,
            logs_event_notify: Arc::new(Notifications::new()),
            new_heads_event_notify: Arc::new(Notifications::new()),
            new_pending_tx_hash_event_notify: Arc::new(Notifications::new()),
            syncing_event_notify: Arc::new(Notifications::new()),
        }
    }

    #[cfg(not(feature = "cluster_redis"))]
    pub fn new(redis_pool: Arc<r2d2::Pool<redis::Client>>, tm_url: &str) -> Self {
        Self {
            tm_url: String::from(tm_url),
            millis: 2000,
            redis_pool,
            logs_event_notify: Arc::new(Notifications::new()),
            new_heads_event_notify: Arc::new(Notifications::new()),
            new_pending_tx_hash_event_notify: Arc::new(Notifications::new()),
            syncing_event_notify: Arc::new(Notifications::new()),
        }
    }

    pub fn start(&self) -> Result<()> {
        let ten_millis = time::Duration::from_millis(self.millis);
        let logs_event_notify = self.logs_event_notify.clone();
        let new_heads_event_notify = self.new_heads_event_notify.clone();
        let new_pending_tx_hash_event_notify = self.new_pending_tx_hash_event_notify.clone();
        let syncing_event_notify = self.syncing_event_notify.clone();

        let redis_pool = self.redis_pool.clone();
        let mut conn = self.redis_pool.get().c(d!())?;
        let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
        let mut last_height = getter.latest_height().c(d!())?;

        let mut last_txhash = vec![];
        let mut last_status = Option::None;

        let tm_url = self.tm_url.clone();
        thread::spawn(move || {
            let tm_url = tm_url;
            loop {
                thread::sleep(ten_millis);

                if let Ok(mut conn) = redis_pool.get() {
                    let mut getter = Getter::new(&mut *conn, PREFIX.to_string());
                    if let Ok(height) = getter.latest_height() {
                        if last_height != height {
                            for h in (last_height + 1)..=height {
                                logs_event_notify.notify(U256::from(h)).unwrap_or_default();
                                new_heads_event_notify
                                    .notify(U256::from(h))
                                    .unwrap_or_default();
                            }
                            last_height = height;
                        }
                    };
                }

                if let Ok(hashs) = get_pending_hash(&tm_url) {
                    for hash in &hashs {
                        if !last_txhash.contains(hash) {
                            new_pending_tx_hash_event_notify
                                .notify(*hash)
                                .unwrap_or_default();
                        }
                    }
                    last_txhash = hashs;
                }
                if let Ok(status) = get_sync_status(&tm_url) {
                    let is_send = match last_status {
                        Some(last) => last != status,
                        None => true,
                    };
                    if is_send {
                        syncing_event_notify.notify(status).unwrap_or_default();
                        last_status = Some(status);
                    }
                }
            }
        });
        Ok(())
    }
}

fn sha2_256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut output = [0u8; 32];
    output.copy_from_slice(&hasher.finalize());
    output
}
fn get_pending_hash(url: &str) -> Result<Vec<H256>> {
    let url = format!("{}/unconfirmed_txs", url);
    let resp = reqwest::blocking::get(url)
        .c(d!())?
        .json::<Value>()
        .c(d!())?;
    let mut pending_hash = vec![];
    if let Some(txs) = resp["result"]["txs"].as_array() {
        for tx in txs {
            if let Some(tx) = tx.as_str() {
                base64::decode(tx)
                    .map(|bytes| {
                        let hasher = sha2_256(&bytes);
                        pending_hash.push(H256::from_slice(&hasher))
                    })
                    .unwrap_or_default();
            }
        }
        Ok(pending_hash)
    } else {
        Err(eg!())
    }
}
fn get_sync_status(url: &str) -> Result<bool> {
    let url = format!("{}/status", url);
    let resp = reqwest::blocking::get(url)
        .c(d!())?
        .json::<Value>()
        .c(d!())?;
    if let Some(catching_up) = resp["result"]["sync_info"]["catching_up"].as_bool() {
        Ok(catching_up)
    } else {
        Err(eg!())
    }
}
