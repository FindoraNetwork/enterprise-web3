use {
    ethereum::TransactionV0 as Transaction,
    serde::{Deserialize, Serialize},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxAction {
    Transact(Transaction),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EthereumAction {
    Ethereum(TxAction),
}
#[derive(Serialize, Deserialize)]
pub struct UncheckedTransaction {
    pub signature: Option<()>,
    pub function: EthereumAction,
}
impl UncheckedTransaction {
    pub fn new_unsigned(tx: Transaction) -> Self {
        Self {
            signature: None,
            function: EthereumAction::Ethereum(TxAction::Transact(tx)),
        }
    }
}

/// EVM_TX_TAG = "evm:"
pub const EVM_TX_TAG: [u8; 4] = [0x65, 0x76, 0x6d, 0x3a];

/// Evm Tx wrapper
pub struct EvmRawTxWrapper {}
impl EvmRawTxWrapper {
    /// wrap
    pub fn wrap(raw_tx: &[u8]) -> Vec<u8> {
        let mut txn_with_tag: Vec<u8> = vec![];
        txn_with_tag.extend_from_slice(&EVM_TX_TAG);
        txn_with_tag.extend_from_slice(raw_tx);

        txn_with_tag
    }
}
