use std::default;

use crate::model::db::init_db;
use crate::model::todo::{TodoPatch, TodoStatus};

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
	assert!(todo_created.id >= 1000, "ID should be >= 1000");
	assert_eq!(data_fx.title.unwrap(), todo_created.title);
	assert_eq!(TodoStatus::Open, todo_created.status);

	Ok(())
}

#[tokio::test]
async fn model_todo_list() -> Result<(), Box<dyn std::error::Error>> {
	// -- FIXTURE
	let db = init_db().await?;

	// -- ACTION

	Ok(())
}
