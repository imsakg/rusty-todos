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

#[test]
fn disk_round_trip_preserves_ids_and_rejects_corruption() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	assert!(Store::load(&path).unwrap().tasks().is_empty());
	let mut store = Store::default();
	store.create("persist me").unwrap();
	store.save(&path).unwrap();
	let mut loaded = Store::load(&path).unwrap();
	assert_eq!(loaded, store);
	assert_eq!(loaded.create("second").unwrap().id, 2);
	std::fs::write(&path, "not JSON").unwrap();
	assert!(Store::load(&path).is_err());
}

#[test]
fn saves_create_parent_directories_and_replace_complete_files() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("nested/todos.json");
	let mut store = Store::default();
	store.create("a long first title").unwrap();
	store.save(&path).unwrap();
	store.rename(1, "short").unwrap();
	store.save(&path).unwrap();
	assert_eq!(Store::load(&path).unwrap(), store);
	assert_eq!(std::fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
	assert!(store.save(dir.path()).is_err());
}

#[test]
fn category_assignment_is_optional_trimmed_and_validated() {
	let mut store = Store::default();
	store.create("work item").unwrap();
	assert_eq!(
		store.set_category(1, Some(" work ")).unwrap().category.as_deref(),
		Some("work")
	);
	assert!(store.set_category(1, Some(" ")).is_err());
	assert_eq!(store.get(1).unwrap().category.as_deref(), Some("work"));
	assert_eq!(store.set_category(1, None).unwrap().category, None);
	assert!(store.set_category(99, Some("work")).is_err());
}

#[test]
fn category_renames_and_removals_update_every_member() {
	let mut store = Store::default();
	for title in ["a", "b"] {
		let id = store.create(title).unwrap().id;
		store.set_category(id, Some("work")).unwrap();
	}
	assert_eq!(store.categories(), vec!["work"]);
	assert!(store.rename_category("work", " ").is_err());
	assert_eq!(store.rename_category("work", "office").unwrap(), 2);
	assert_eq!(store.categories(), vec!["office"]);
	assert_eq!(store.remove_category("office").unwrap(), 2);
	assert!(store.categories().is_empty());
	assert!(store.remove_category("missing").is_err());
}

#[test]
fn priorities_default_to_normal_and_serialize_as_names() {
	let mut store = Store::default();
	assert_eq!(store.create("normal").unwrap().priority, Priority::Normal);
	assert_eq!(
		store.set_priority(1, Priority::Urgent).unwrap().priority,
		Priority::Urgent
	);
	assert_eq!(serde_json::to_string(&Priority::High).unwrap(), "\"high\"");
	assert!(serde_json::from_str::<Priority>("\"impossible\"").is_err());
	assert!(store.set_priority(99, Priority::Low).is_err());
}

#[test]
fn due_dates_can_be_set_and_cleared_with_leap_day_validation() {
	let mut store = Store::default();
	store.create("scheduled").unwrap();
	let due = parse_date("2024-02-29").unwrap();
	assert!(parse_date("2023-02-29").is_err());
	assert!(parse_date("2024-2-9").is_err());
	assert_eq!(store.set_due(1, Some(due)).unwrap().due, Some(due));
	assert_eq!(store.set_due(1, None).unwrap().due, None);
}

#[test]
fn reminders_respect_time_completion_and_acknowledgment() {
	let mut store = Store::default();
	store.create("remind me").unwrap();
	let now = "2024-06-03T18:00:00Z".parse().unwrap();
	store.set_reminder(1, Some(now)).unwrap();
	assert!(store
		.pending_reminders("2024-06-03T17:59:59Z".parse().unwrap())
		.is_empty());
	assert_eq!(store.pending_reminders(now).len(), 1);
	store.set_status(1, Status::Done).unwrap();
	assert!(store.pending_reminders(now).is_empty());
	store.set_status(1, Status::Open).unwrap();
	store.dismiss_reminder(1).unwrap();
	assert!(store.pending_reminders(now).is_empty());
	store.set_reminder(1, Some(now)).unwrap();
	assert_eq!(store.pending_reminders(now).len(), 1);
	store.set_reminder(1, None).unwrap();
	assert!(store.dismiss_reminder(1).is_err());
}

#[test]
fn query_combines_status_category_and_priority_filters() {
	let mut store = Store::default();
	store.create("a").unwrap();
	store.create("b").unwrap();
	store.create("c").unwrap();
	store.set_category(1, Some("work")).unwrap();
	store.set_category(2, Some("work")).unwrap();
	store.set_priority(1, Priority::High).unwrap();
	store.set_status(2, Status::Done).unwrap();
	let query = Query {
		status: Some(Status::Open),
		category: Some("work".into()),
		priority: Some(Priority::High),
		..Default::default()
	};
	assert_eq!(store.query(&query).iter().map(|t| t.id).collect::<Vec<_>>(), vec![1]);
	assert_eq!(
		store.query(&Query {
			uncategorized: true,
			..Default::default()
		})[0]
			.id,
		3
	);
}

#[test]
fn search_and_due_filters_exclude_unscheduled_and_completed_overdue_tasks() {
	let mut store = Store::default();
	for title in ["Write REPORT", "report later", "read"] {
		store.create(title).unwrap();
	}
	store.set_due(1, Some(parse_date("2024-06-01").unwrap())).unwrap();
	store.set_due(2, Some(parse_date("2024-06-03").unwrap())).unwrap();
	let as_of = parse_date("2024-06-03").unwrap();
	let query = Query {
		search: Some("report".into()),
		overdue_as_of: Some(as_of),
		..Default::default()
	};
	assert_eq!(store.query(&query).iter().map(|t| t.id).collect::<Vec<_>>(), vec![1]);
	store.set_status(1, Status::Done).unwrap();
	assert!(store.query(&query).is_empty());
	assert_eq!(
		store
			.query(&Query {
				due_before: Some(as_of),
				..Default::default()
			})
			.len(),
		2
	);
}

#[test]
fn sorting_is_deterministic_and_keeps_missing_due_dates_last() {
	let mut store = Store::default();
	for title in ["Zebra", "alpha", "alpha"] {
		store.create(title).unwrap();
	}
	let title = Query {
		sort: SortField::Title,
		..Default::default()
	};
	assert_eq!(
		store.query(&title).iter().map(|t| t.id).collect::<Vec<_>>(),
		vec![2, 3, 1]
	);
	store.set_due(2, Some(parse_date("2024-06-01").unwrap())).unwrap();
	store.set_due(3, Some(parse_date("2024-06-02").unwrap())).unwrap();
	let due = Query {
		sort: SortField::Due,
		descending: true,
		..Default::default()
	};
	assert_eq!(
		store.query(&due).iter().map(|t| t.id).collect::<Vec<_>>(),
		vec![3, 2, 1]
	);
	store.set_priority(2, Priority::Urgent).unwrap();
	assert_eq!(
		store.query(&Query {
			sort: SortField::Priority,
			descending: true,
			..Default::default()
		})[0]
			.id,
		2
	);
}
