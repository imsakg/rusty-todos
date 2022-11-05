use crate::{
	model::{Db, TodoMac},
	security::{utx_from_token, UserCtx},
};
use serde_json::json;
use std::{convert::Infallible, sync::Arc};
use warp::{reply::Json, Filter, Rejection};

use super::{filter_auth::do_auth, filter_utils::with_db};

pub fn todo_rest_filters(
	base_path: &'static str,
	db: Arc<Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
	let todos_path = warp::path(base_path).and(warp::path("todos")); // /api/todos
	let common = with_db(db.clone()).and(do_auth(db.clone()));

	// LIST todos `GET todos/`
	let list = todos_path
		.and(warp::get())
		.and(warp::path::end())
		.and(common.clone())
		.and_then(todo_list);

	list
}

async fn todo_list(db: Arc<Db>, utx: UserCtx) -> Result<Json, warp::Rejection> {
	// FIXME: Add proper error handling
	let todos = TodoMac::list(&db, &utx).await?;

	let response = json!({ "data": todos });

	Ok(warp::reply::json(&response))
}

/* #region  Test */
#[cfg(test)]
#[path = "../_tests/web_todo.rs"]
mod tests;
/* #endregion */
