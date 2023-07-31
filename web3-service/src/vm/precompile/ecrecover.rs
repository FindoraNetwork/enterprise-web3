use {
    super::{ensure_linear_cost, PrecompileResult},
    evm::executor::stack::PrecompileOutput,
    evm_runtime::{Context, ExitSucceed},
    ruc::{eg, Result},
    std::cmp::min,
    tiny_keccak::{Hasher, Keccak},
};

/// The ecrecover precompile.
pub struct ECRecover;

impl ECRecover {
    const BASE: u64 = 3000;
    const WORD: u64 = 0;

    pub fn execute(
        i: &[u8],
        gas_limit: Option<u64>,
        _context: &Context,
        _is_static: bool,
    ) -> PrecompileResult {
        let mut input = [0u8; 128];
        input[..min(i.len(), 128)].copy_from_slice(&i[..min(i.len(), 128)]);

        let mut msg = [0u8; 32];
        let mut sig = [0u8; 65];

        msg[0..32].copy_from_slice(&input[0..32]);
        sig[0..32].copy_from_slice(&input[64..96]);
        sig[32..64].copy_from_slice(&input[96..128]);
        sig[64] = input[63];

        let result = match secp256k1_ecdsa_recover(&sig, &msg) {
            Ok(pubkey) => {
                let mut address = keccak_256(&pubkey);
                address[0..12].copy_from_slice(&[0u8; 12]);
                address.to_vec()
            }
            Err(_) => [0u8; 0].to_vec(),
        };
        let cost = ensure_linear_cost(gas_limit, i.len() as u64, Self::BASE, Self::WORD)?;

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost,
            output: result,
            logs: vec![],
        })
    }
}

pub fn secp256k1_ecdsa_recover(sig: &[u8; 65], msg: &[u8; 32]) -> Result<[u8; 64]> {
    let rs = libsecp256k1::Signature::parse_standard_slice(&sig[0..64])
        .map_err(|_| eg!("Ecdsa signature verify error: bad RS"))?;
    let v = libsecp256k1::RecoveryId::parse(if sig[64] > 26 { sig[64] - 27 } else { sig[64] })
        .map_err(|_| eg!("Ecdsa signature verify error: bad V"))?;
    let pubkey = libsecp256k1::recover(&libsecp256k1::Message::parse(msg), &rs, &v)
        .map_err(|_| eg!("Ecdsa signature verify error: bad signature"))?;
    let mut res = [0u8; 64];
    res.copy_from_slice(&pubkey.serialize()[1..65]);
    Ok(res)
}

/// Do a keccak 256-bit hash and return result.
pub fn keccak_256(data: &[u8]) -> [u8; 32] {
    let mut keccak = Keccak::v256();
    keccak.update(data);
    let mut output = [0u8; 32];
    keccak.finalize(&mut output);
    output
}
