use {
    crate::error::Result,
    ethereum::{LegacyTransaction, LegacyTransactionMessage},
    ethereum_types::{H160, H256},
    sha3::{Digest, Keccak256},
};

pub fn public_key(tx: &LegacyTransaction) -> Result<[u8; 64]> {
    let mut sig = [0u8; 65];
    let mut msg = [0u8; 32];
    sig[0..32].copy_from_slice(&tx.signature.r()[..]);
    sig[32..64].copy_from_slice(&tx.signature.s()[..]);
    sig[64] = tx.signature.standard_v();
    msg.copy_from_slice(&LegacyTransactionMessage::from(tx.clone()).hash()[..]);
    let rs = libsecp256k1::Signature::parse_standard_slice(&sig[0..64])?;
    let v = libsecp256k1::RecoveryId::parse(if sig[64] > 26 { sig[64] - 27 } else { sig[64] })?;
    let pubkey = libsecp256k1::recover(&libsecp256k1::Message::parse(&msg), &rs, &v)?;
    let mut res = [0u8; 64];
    res.copy_from_slice(&pubkey.serialize()[1..65]);
    Ok(res)
}

pub fn recover_signer(transaction: &LegacyTransaction) -> Result<H160> {
    let pubkey = public_key(transaction)?;
    Ok(H160::from(H256::from_slice(
        Keccak256::digest(&pubkey).as_slice(),
    )))
}
