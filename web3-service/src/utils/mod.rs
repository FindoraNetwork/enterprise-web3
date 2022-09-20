use {evm_exporter::Getter, redis::ConnectionLike, ruc::*, web3_rpc_core::types::BlockNumber};

pub fn block_number_to_height<C: ConnectionLike>(
    bn: Option<BlockNumber>,
    getter: &mut Getter<C>,
) -> Result<u32> {
    let bn = if let Some(bn) = bn {
        bn
    } else {
        BlockNumber::Latest
    };

    let height = match bn {
        BlockNumber::Hash {
            hash,
            require_canonical: _,
        } => getter
            .get_height_by_block_hash(hash)
            .c(d!())?
            .ok_or(eg!("hash not find"))?
            .as_u32(),
        BlockNumber::Num(num) => num as u32,
        BlockNumber::Latest => getter.latest_height().c(d!())?,
        BlockNumber::Earliest => 1,
        BlockNumber::Pending => getter.latest_height().c(d!())?,
    };

    Ok(height)
}
