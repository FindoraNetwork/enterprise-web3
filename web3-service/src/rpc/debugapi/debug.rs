use {
    super::types::{TraceParams, TransactionTrace},
    ethereum_types::H256,
    jsonrpc_core::Result,
    jsonrpc_derive::rpc,
    web3_rpc_core::types::{BlockNumber, CallRequest},
};

#[rpc(server)]
pub trait DebugApi {
    #[rpc(name = "debug_traceBlockByNumber")]
    fn trace_block_by_number(
        &self,
        _: BlockNumber,
        _: Option<TraceParams>,
    ) -> Result<Vec<TransactionTrace>>;
    #[rpc(name = "debug_traceBlockByHash")]
    fn trace_block_by_hash(&self, _: H256, _: Option<TraceParams>)
        -> Result<Vec<TransactionTrace>>;
    #[rpc(name = "debug_traceCall")]
    fn trace_call(
        &self,
        _: CallRequest,
        _: BlockNumber,
        _: Option<TraceParams>,
    ) -> Result<TransactionTrace>;
    #[rpc(name = "debug_traceTransaction")]
    fn trace_transaction(&self, _: H256, _: Option<TraceParams>) -> Result<TransactionTrace>;
}
