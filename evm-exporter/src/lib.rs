mod error;
pub use error::*;

mod types;
pub use types::*;

pub mod keys;

mod getter;
pub use getter::*;

mod setter;
pub use setter::*;

mod utils;
pub use utils::*;

pub enum ConnectionType {
    #[cfg(feature = "redis")]
    Redis(String),
    #[cfg(feature = "redis-cluster")]
    RedisCluster(Vec<String>),
    #[cfg(feature = "postgres")]
    Postgres(String),
}
