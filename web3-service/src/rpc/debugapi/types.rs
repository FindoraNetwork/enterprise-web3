use {
    ethereum_types::{H256, U256},
    evm::Opcode,
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceParams {
    pub disable_storage: Option<bool>,
    pub disable_memory: Option<bool>,
    pub disable_stack: Option<bool>,
    pub tracer: Option<String>,
    pub timeout: Option<String>,
}
#[derive(Clone, Serialize)]
pub enum CallType {
    Call,
    CallCode,
    DelegateCall,
    StaticCall,
}

pub enum ContextType {
    Call(CallType),
    Create,
}

impl ContextType {
    pub fn from(opcode: Opcode) -> Option<Self> {
        match opcode {
            Opcode::CREATE | Opcode::CREATE2 => Some(ContextType::Create),
            Opcode::CALL => Some(ContextType::Call(CallType::Call)),
            Opcode::CALLCODE => Some(ContextType::Call(CallType::CallCode)),
            Opcode::DELEGATECALL => Some(ContextType::Call(CallType::DelegateCall)),
            Opcode::STATICCALL => Some(ContextType::Call(CallType::StaticCall)),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize)]
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

#[derive(Serialize)]
pub struct TransactionTrace {
    pub(crate) gas: U256,
    pub(crate) return_value: Vec<u8>,
    pub(crate) step_logs: Vec<RawStepLog>,
}
