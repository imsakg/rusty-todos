use rusty_todos::local::*;

#[test]
fn titles_are_trimmed_and_blank_titles_rejected() {
	assert_eq!(validate_title("  first task  ").unwrap(), "first task");
	assert!(validate_title(" \n\t").is_err());
	assert!(validate_title(&"x".repeat(501)).is_err());
}

#[test]
fn create_allocates_stable_ids_without_consuming_them_on_errors() {
	let mut store = Store::default();
	assert!(store.create(" ").is_err());
	assert_eq!(store.create(" first ").unwrap().id, 1);
	let second = store.create("second").unwrap();
	assert_eq!(second.id, 2);
	assert_eq!(second.status, Status::Open);
}

#[test]
fn get_and_list_return_only_existing_tasks() {
	let mut store = Store::default();
	assert!(store.tasks().is_empty());
	let task = store.create("read me").unwrap();
	assert_eq!(store.get(task.id).unwrap(), &task);
	assert_eq!(store.tasks(), &[task]);
	assert!(matches!(store.get(999), Err(Error::NotFound(999))));
}

#[test]
fn rename_is_validated_before_mutating_the_task() {
	let mut store = Store::default();
	let id = store.create("before").unwrap().id;
	assert!(store.rename(id, " ").is_err());
	assert_eq!(store.get(id).unwrap().title, "before");
	assert_eq!(store.rename(id, " after ").unwrap().title, "after");
	assert!(store.rename(99, "missing").is_err());
}

#[test]
fn deleted_ids_are_not_recycled() {
	let mut store = Store::default();
	let task = store.create("temporary").unwrap();
	assert_eq!(store.delete(task.id).unwrap(), task);
	assert!(store.get(task.id).is_err());
	assert!(store.delete(task.id).is_err());
	assert_eq!(store.create("next").unwrap().id, 2);
}

#[test]
fn complete_and_reopen_are_idempotent() {
	let mut store = Store::default();
	let id = store.create("finish").unwrap().id;
	assert_eq!(store.set_status(id, Status::Done).unwrap().status, Status::Done);
	assert_eq!(store.set_status(id, Status::Done).unwrap().status, Status::Done);
	assert_eq!(store.set_status(id, Status::Open).unwrap().status, Status::Open);
	assert!(store.set_status(99, Status::Done).is_err());
}
