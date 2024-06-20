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
    Redis(redis::Connection),
    Postgres(sqlx::PgConnection),
}
