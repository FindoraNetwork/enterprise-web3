use {evm_exporter::Getter, ruc::*, web3_rpc_core::types::BlockNumber};

pub fn block_number_to_height(
    block_number: Option<BlockNumber>,
    getter: &mut dyn Getter,
) -> Result<u32> {
    let height = match block_number.unwrap_or(BlockNumber::Latest) {
        BlockNumber::Hash {
            hash,
            require_canonical: _,
        } => getter
            .get_height_by_block_hash(hash)
            .map_err(|e| eg!(e))?
            .ok_or(eg!("hash not find"))?
            .as_u32(),
        BlockNumber::Num(num) => num as u32,
        BlockNumber::Latest => getter.latest_height().map_err(|e| eg!(e))?,
        BlockNumber::Earliest => 1,
        BlockNumber::Pending => getter.latest_height().map_err(|e| eg!(e))?,
    };

    Ok(height)
}
