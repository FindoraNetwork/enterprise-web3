use crate::vm::EthVmBackend;
use evm::backend::Backend;
use ovr_ruc::*;
use primitive_types::{H256, U256};
use web3_rpc_core::types::BlockNumber;

pub fn block_number_to_height(bn: Option<BlockNumber>, backend: &EthVmBackend) -> Result<u32> {
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
            let mut getter = backend.gen_getter().c(d!())?;
            if let Some(h) = getter.get_height_by_block_hash(hash).c(d!())? {
                h.as_u32()
            } else {
                0
            }
        }
        BlockNumber::Num(num) => num as u32,
        BlockNumber::Latest => {
            let mut getter = backend.gen_getter().c(d!())?;
            getter.latest_height().c(d!())?
        }
        BlockNumber::Earliest => 1,
        BlockNumber::Pending => 0,
    };

    Ok(height)
}
