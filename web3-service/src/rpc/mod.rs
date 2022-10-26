pub mod eth;
pub mod eth_filter;
pub mod eth_pubsub;
pub mod health;
pub mod net;
pub mod web3;

const MAX_PAST_LOGS: u32 = 10000;
const MAX_STORED_FILTERS: usize = 500;

pub fn internal_err<T: ToString>(message: T) -> jsonrpc_core::Error {
    jsonrpc_core::Error {
        code: jsonrpc_core::ErrorCode::InternalError,
        message: message.to_string(),
        data: None,
    }
}
