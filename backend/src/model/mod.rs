use std::error;
use thiserror::Error as ThisError;

mod db;
mod todo;

// re-export
pub use db::init_db;
pub use db::Db;
pub use todo::TodoMac;

#[derive(ThisError, Debug)]
pub enum Error {
	#[error("Entity Not Found! - {0}[{1}] ")]
	EntityNotFound(&'static str, String),

	#[error(transparent)]
	SqlxError(#[from] sqlx::Error),

	#[error(transparent)]
	IOError(#[from] std::io::Error),
}
