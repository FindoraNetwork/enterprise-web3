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

pub enum ConnectionType<'a> {
    #[cfg(feature = "redis")]
    Redis(&'a str),
    #[cfg(feature = "redis-cluster")]
    RedisCluster(&'a [&'a str]),
    #[cfg(feature = "postgres")]
    Postgres(&'a str),
}
