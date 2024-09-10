use {evm_exporter::Getter, ruc::*, std::sync::Arc, web3_rpc_core::types::BlockNumber};

pub fn block_number_to_height(
    block_number: Option<BlockNumber>,
    getter: Arc<dyn Getter>,
) -> Result<u32> {
    let height = match block_number.unwrap_or(BlockNumber::Latest) {
        BlockNumber::Hash { hash, .. } => {
            let height_result =
                tokio::task::block_in_place(move || getter.get_height_by_block_hash(hash))
                    .map_err(|e| {
                        eg!(format!(
                    "block_number_to_height block_in_place get_height_by_block_hash error: {:?}",
                    e.to_string()
                ))
                    })?;

            height_result.c(d!("hash not find"))?.as_u32()
        }
        BlockNumber::Num(num) => num as u32,
        BlockNumber::Latest => {
            let latest_height_result = tokio::task::block_in_place(move || getter.latest_height())
                .map_err(|e| {
                    eg!(format!(
                        "block_number_to_height block_in_place latest_height error: {:?}",
                        e.to_string()
                    ))
                })?;

            latest_height_result
        }
        BlockNumber::Earliest => 1,
        BlockNumber::Pending => {
            let latest_height_result = tokio::task::block_in_place(move || getter.latest_height())
                .map_err(|e| {
                    eg!(format!(
                        "block_number_to_height block_in_place latest_height error: {:?}",
                        e.to_string()
                    ))
                })?;

            latest_height_result
        }
    };

    Ok(height)
}
