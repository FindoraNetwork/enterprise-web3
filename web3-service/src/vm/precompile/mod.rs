use std::cell::RefCell;

mod anemoi;
mod blake2f;
mod bn128;
mod ecrecover;
mod frc20;
mod identity;
mod modexp;
mod ripemd160_precompile;
mod sha256;

use {
    anemoi::Anemoi,
    blake2f::Blake2F,
    bn128::{Bn128Add, Bn128Mul, Bn128Pairing},
    ecrecover::ECRecover,
    ethereum_types::H160,
    evm::executor::stack::{PrecompileFailure, PrecompileFn, PrecompileOutput, PrecompileSet},
    evm_exporter::Getter,
    evm_runtime::{Context, ExitError},
    frc20::FRC20,
    identity::Identity,
    modexp::Modexp,
    once_cell::sync::{Lazy, OnceCell},
    ripemd160_precompile::Ripemd160,
    ruc::*,
    sha256::Sha256,
    std::{collections::BTreeMap, sync::Arc},
};

pub static GETTER: OnceCell<Arc<dyn Getter + Send + Sync>> = OnceCell::new();

pub type PrecompileResult = core::result::Result<PrecompileOutput, PrecompileFailure>;

pub(crate) static PRECOMPILE_SET: Lazy<BTreeMap<H160, PrecompileFn>> = Lazy::new(|| {
    map! {B
        idx_to_h160(1) => ECRecover::execute as PrecompileFn,
        idx_to_h160(2) => Sha256::execute,
        idx_to_h160(3) => Ripemd160::execute,
        idx_to_h160(4) => Identity::execute,
        idx_to_h160(5) => Modexp::execute,
        idx_to_h160(6) => Bn128Add::execute,
        idx_to_h160(7) => Bn128Mul::execute,
        idx_to_h160(8) => Bn128Pairing::execute,
        idx_to_h160(9) => Blake2F::execute,
        // idx_to_h160(0x1000) => FRC20::execute,
        idx_to_h160(0x2002) => Anemoi::execute,
    }
});

#[inline(always)]
pub(crate) fn idx_to_h160(i: u64) -> H160 {
    H160::from_low_u64_be(i)
}

pub struct Web3EvmPrecompiles {
    frc20: RefCell<FRC20>,
}
impl Web3EvmPrecompiles {
    pub fn new(height: u32) -> Self {
        Web3EvmPrecompiles {
            frc20: RefCell::new(FRC20::new(height)),
        }
    }
}

impl PrecompileSet for Web3EvmPrecompiles {
    fn execute(
        &self,
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> Option<core::result::Result<PrecompileOutput, PrecompileFailure>> {
        if address == H160::from_low_u64_be(FRC20::contract_id()) {
            Some(
                self.frc20
                    .borrow_mut()
                    .execute(input, target_gas, context, is_static),
            )
        } else {
            PRECOMPILE_SET
                .get(&address)
                .map(|f| f(input, target_gas, context, is_static))
        }
    }

    fn is_precompile(&self, address: H160) -> bool {
        std::vec![1, 2, 3, 4, 5, 1024, 1025]
            .into_iter()
            .map(idx_to_h160)
            .any(|x| x == address)
    }
}

fn ensure_linear_cost(
    gas_limit: Option<u64>,
    len: u64,
    base: u64,
    word: u64,
) -> std::result::Result<u64, PrecompileFailure> {
    let cost = base
        .checked_add(word.checked_mul(len.saturating_add(31) / 32).ok_or(
            PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            },
        )?)
        .ok_or(PrecompileFailure::Error {
            exit_status: ExitError::OutOfGas,
        })?;

    if let Some(target_gas) = gas_limit {
        if cost > target_gas {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::OutOfGas,
            });
        }
    }

    Ok(cost)
}
