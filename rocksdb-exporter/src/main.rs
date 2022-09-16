mod evm_rocksdb_storage;

use {
    evm_exporter::{Getter, Setter, PREFIX},
    evm_rocksdb_storage::{
        evm_rocksdb::RocksDB, get_account_info, get_block_info, get_current_height,
    },
    primitive_types::U256,
    redis::Client,
    ruc::*,
    std::{cmp::Ordering, sync::Arc},
};

fn main() {
    let path = "/tmp/state.db";
    let statedb = Arc::new(pnk!(RocksDB::open(path)));
    let path = "/tmp/history.db";
    let hisdb = Arc::new(pnk!(RocksDB::open(path)));
    let client = pnk!(Client::open("redis://127.0.0.1:6379"));
    let mut setter = Setter::new(pnk!(client.get_connection()), PREFIX.to_string());
    let current_height = pnk!(get_current_height(&hisdb));
    // let mut getter = Getter::new_genesis(pnk!(client.get_connection()), PREFIX.to_string());
    // let mut height = U256::from(pnk!(getter.get_height()));
    pnk!(setter.clear());
    let mut height = U256::zero();

    println!("start height:{:?},stop height:{:?}", height, current_height);

    loop {
        height = height.saturating_add(U256::one());
        let info = pnk!(get_block_info(height, &hisdb));
        if let Some((block, receipts, statuses, transaction_index)) = info {
            pnk!(setter.set_block_info(block.into(), receipts, statuses, transaction_index));
        } else {
            println!("jump over height:{:?}", height);
            continue;
        }

        let (accountstores, codes, account_storages) = pnk!(get_account_info(
            &statedb,
            if Ordering::Equal == height.cmp(&current_height) {
                0
            } else {
                height.as_u64()
            },
        ));

        pnk!(setter.set_account_basic(height.as_u32(), accountstores));
        pnk!(setter.set_codes(height.as_u32(), codes));
        pnk!(setter.set_account_storages(height.as_u32(), account_storages));
        pnk!(setter.set_height(height.as_u32()));

        println!("complete height:{:?}", height);
        if Ordering::Greater == height.cmp(&current_height) {
            break;
        }
    }
}
