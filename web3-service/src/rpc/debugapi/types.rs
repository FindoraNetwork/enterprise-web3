use {
    ethereum_types::{H256, U256},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct TraceParams {
    pub disable_storage: Option<bool>,
    pub disable_memory: Option<bool>,
    pub disable_stack: Option<bool>,
    pub tracer: Option<String>,
    pub timeout: Option<String>,
}

#[derive(Serialize)]
pub struct TransactionTrace {
    pub gas: U256,
    pub return_value: Vec<u8>,
    pub step_logs: Vec<RawStepLog>,
}

#[derive(Debug, Serialize)]
pub struct RawStepLog {
    pub depth: U256,
    pub gas: U256,
    pub gas_cost: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<Vec<H256>>,
    pub op: u8,
    pub pc: U256,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<Vec<H256>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<BTreeMap<H256, H256>>,
}
