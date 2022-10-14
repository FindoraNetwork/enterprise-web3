use {jsonrpc_core::Result, jsonrpc_derive::rpc, serde::Serialize};

#[derive(Debug, Serialize)]
pub struct Health {
    pub code: i32,
    pub message: String,
    pub data: Option<()>,
}
#[rpc(server)]
pub trait HealthApi {
    /// Returns protocol version.
    #[rpc(name = "system_health")]
    fn system_health(&self) -> Result<Health>;
}

pub struct HealthApiImpl;

impl HealthApiImpl {
    pub fn new() -> Self {
        Self
    }
}

impl HealthApi for HealthApiImpl {
    fn system_health(&self) -> Result<Health> {
        Ok(Health {
            code: 0,
            message: String::new(),
            data: None,
        })
    }
}
