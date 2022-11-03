use std::default;

use crate::model::db::init_db;
use crate::model::todo::TodoPatch;

use super::TodoMac;

#[tokio::test]
async fn model_todo_create() -> Result<(), Box<dyn std::error::Error>> {
	// -- FIXTURE
	let db = init_db().await?;
	let data_fx = TodoPatch {
		title: Some("test - model_todo_create 1".to_string()),
		..Default::default()
	};

	// -- ACTION
	let todo_created = TodoMac::create(&db, data_fx.clone()).await?;

	// -- CHECK
	println!("\n\n->> {:?}", todo_created);

	Ok(())
}

#[tokio::test]
async fn model_todo_list() -> Result<(), Box<dyn std::error::Error>> {
	// -- FIXTURE
	let db = init_db().await?;

	// -- ACTION

	Ok(())
}
