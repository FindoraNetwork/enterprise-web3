mod config;
mod evm_rocksdb_storage;

use {
    config::Config,
    evm_exporter::{ConnectionType, Getter, Setter, PREFIX},
    evm_rocksdb_storage::{
        evm_rocksdb::RocksDB, get_account_info, get_block_info, get_current_height,
    },
    primitive_types::U256,
    ruc::*,
    std::{cmp::Ordering, sync::Arc},
};

fn main() {
    let config_path = pnk!(std::env::var("EXPORT_CONFIG_FILE_PATH"));
    let config = pnk!(Config::new(&config_path));

    let statedb = Arc::new(pnk!(RocksDB::open(config.state_db_path.as_str())));
    let hisdb = Arc::new(pnk!(RocksDB::open(config.history_db_path.as_str())));

    #[cfg(feature = "redis-cluster")]
    let mut setter = Setter::new(
        ConnectionType::RedisCluster(&config.redis_url),
        PREFIX.to_string(),
    );
    #[cfg(feature = "redis-cluster")]
    let mut getter = Getter::new(
        ConnectionType::RedisCluster(&config.redis_url),
        PREFIX.to_string(),
    );
    #[cfg(feature = "redis")]
    let mut setter = Setter::new(
        ConnectionType::Redis(&config.redis_url[0]),
        PREFIX.to_string(),
    );
    #[cfg(feature = "redis")]
    let mut getter = Getter::new(
        ConnectionType::Redis(&config.redis_url[0]),
        PREFIX.to_string(),
    );
    #[cfg(feature = "postgres")]
    let mut setter = Setter::new(
        ConnectionType::Postgres(&config.postgres_uri),
        String::new(),
    );
    #[cfg(feature = "postgres")]
    let mut getter = Getter::new(
        ConnectionType::Postgres(&config.postgres_uri),
        String::new(),
    );

    let current_height = pnk!(get_current_height(&hisdb));

    let mut height = if config.clear {
        pnk!(setter.clear());
        U256::zero()
    } else {
        U256::from(pnk!(getter.latest_height()))
    };

    println!("start height:{:?},stop height:{:?}", height, current_height);

    loop {
        height = height.saturating_add(U256::one());
        if Ordering::Greater == height.cmp(&current_height) {
            break;
        }
        let info = pnk!(get_block_info(height, &hisdb));
        if let Some((block, receipts, statuses)) = info {
            let receipts = receipts
                .into_iter()
                .map(ethereum::ReceiptAny::Frontier)
                .collect::<Vec<_>>();
            pnk!(setter.set_block_info(block.into(), receipts, statuses));
        } else {
            println!("jump over height:{:?}", height);
            continue;
        };

        let (accountstores, codes, accounts, allowances, total_issuance) = pnk!(get_account_info(
            &statedb,
            if Ordering::Equal == height.cmp(&current_height) {
                0
            } else {
                height.as_u64()
            },
        ));

        let h = height.as_u32();
        for (address, (nonce, balance)) in accountstores {
            pnk!(setter.set_balance(h, address, balance));
            pnk!(setter.set_nonce(h, address, nonce));
        }
        for (address, code) in codes {
            pnk!(setter.set_byte_code(h, address, code));
        }
        for ((address, index), value) in accounts {
            pnk!(setter.set_state(h, address, index, value));
        }

        for ((owner, spender), value) in allowances {
            pnk!(setter.set_allowances(h, owner, spender, value));
        }
        pnk!(setter.set_total_issuance(h, total_issuance));

        pnk!(setter.set_height(height.as_u32()));

        println!("complete height:{:?}", height);
    }
}
