use crate::vm::EthVmBackend;
use evm::backend::Backend;
use primitive_types::H256;
use web3_rpc_core::types::BlockNumber;

pub fn block_number_to_height(
    bn: Option<BlockNumber>,
    backend: &EthVmBackend,
) -> anyhow::Result<u32> {
    let bn = if let Some(bn) = bn {
        bn
    } else {
        BlockNumber::Latest
    };

    let height = match bn {
        BlockNumber::Hash {
            hash,
            require_canonical: _,
        } => {
            if let Some(h) = backend.hash_height_map.get(&hash) {
                *h
            } else {
                return Err(anyhow::Error::msg(""));
            }
        }
        BlockNumber::Num(num) => num as u32,
        BlockNumber::Latest => {
            let mut getter = backend.gen_getter(None)?;
            getter.latest_height()?
        }
        BlockNumber::Earliest => 1,
        BlockNumber::Pending => 0,
    };

    Ok(height)
}
