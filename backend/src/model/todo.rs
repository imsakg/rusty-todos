use super::db::Db;

/* #region  Todo Types */
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Todo {
	pub id: i64,
	pub cid: i64,
	pub title: String,
}
/* #endregion */

/* #region  TodoMAC */
pub struct TodoMac;

impl TodoMac {
	pub async fn list(db: &Db) -> Result<Vec<Todo>, sqlx::Error> {
		let sql = "SELECT id, cid, title, FROM todo OWNER BY id DESC";

		// build the sqlx-query
		let query = sqlx::query_as::<_, Todo>(&sql);
		// execute the query
		let todos = query.fetch_all(db).await?;

		Ok(todos)
	}
}
/* #endregion */

#[cfg(test)]
#[path = "../_tests/model_todo.rs"]
mod tests;
