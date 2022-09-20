pub mod eth;
pub mod net;
pub mod web3;

pub fn internal_err<T: ToString>(message: T) -> jsonrpc_core::Error {
    jsonrpc_core::Error {
        code: jsonrpc_core::ErrorCode::InternalError,
        message: message.to_string(),
        data: None,
    }
}
