use std::error;
use thiserror::Error as ThisError;

use crate::model::Db;

pub struct UserCtx {
	pub user_id: i64,
}

pub async fn utx_from_token(_db: &Db, token: &str) -> Result<UserCtx, Error> {
	// TODO: real validation needed
	// for now, just parse to i64
	match token.parse::<i64>() {
		Ok(user_id) => Ok(UserCtx { user_id }),
		Err(_) => Err(Error::InvalidToken(token.to_string())),
	}
}

#[derive(ThisError, Debug)]
pub enum Error {
	#[error("Invalid token {0}")]
	InvalidToken(String),
}
