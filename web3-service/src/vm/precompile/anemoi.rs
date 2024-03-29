use evm::executor::stack::{PrecompileFailure, PrecompileOutput};
use evm_runtime::{Context, ExitError, ExitSucceed};
use noah_algebra::{bls12_381::BLSScalar, prelude::Scalar};
use noah_crypto::basic::anemoi_jive::{AnemoiJive, AnemoiJive381};

use super::PrecompileResult;

/// The Anemoi precompile
pub struct Anemoi;

impl Anemoi {
    pub const GAS_PER_PERM: u64 = 125000;

    pub fn execute(
        input: &[u8],
        gas_limit: Option<u64>,
        _context: &Context,
        _is_static: bool,
    ) -> PrecompileResult {
        if input.len() % 32 != 0 {
            return Err(PrecompileFailure::Error {
                exit_status: ExitError::Other("input must be multiplies of 32 bytes".into()),
            });
        }

        let num_elems = input.len() / 32;

        let gas_cost = ((num_elems + 2) / 3) as u64 * Self::GAS_PER_PERM;
        if let Some(gas_left) = gas_limit {
            if gas_left < gas_cost {
                return Err(PrecompileFailure::Error {
                    exit_status: ExitError::OutOfGas,
                });
            }
        };

        let mut field_elems = Vec::with_capacity(num_elems);

        for i in 0..num_elems {
            let res = BLSScalar::from_bytes(
                &input[i * 32..(i + 1) * 32]
                    .iter()
                    .rev()
                    .copied()
                    .collect::<Vec<u8>>(),
            );
            if let Ok(res) = res {
                field_elems.push(res);
            } else {
                return Err(PrecompileFailure::Error {
                    exit_status: ExitError::Other("Cannot convert bytes to field elements".into()),
                });
            }
        }

        let mut res = AnemoiJive381::eval_variable_length_hash(&field_elems).to_bytes();
        res.reverse();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_cost,
            output: res.to_vec(),
            logs: Default::default(),
        })
    }
}
