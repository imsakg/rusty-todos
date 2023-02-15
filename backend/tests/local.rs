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

#[test]
fn pagination_runs_after_sorting_and_handles_empty_pages() {
	let mut store = Store::default();
	for title in ["c", "a", "b"] {
		store.create(title).unwrap();
	}
	let page = Query {
		sort: SortField::Title,
		offset: 1,
		limit: Some(1),
		..Default::default()
	};
	assert_eq!(store.query(&page)[0].title, "b");
	assert!(store
		.query(&Query {
			offset: usize::MAX,
			..Default::default()
		})
		.is_empty());
	assert!(store
		.query(&Query {
			limit: Some(0),
			..Default::default()
		})
		.is_empty());
}

#[test]
fn json_exports_are_versioned_and_support_round_trips() {
	let mut store = Store::default();
	store.create("backup").unwrap();
	store.set_category(1, Some("home")).unwrap();
	let exported = store.export_json().unwrap();
	assert_eq!(
		serde_json::from_str::<serde_json::Value>(&exported).unwrap()["schema_version"],
		1
	);
	assert_eq!(Store::from_json(&exported).unwrap(), store);
	assert!(Store::from_json(&exported.replace("\"schema_version\": 1", "\"schema_version\": 2")).is_err());
	let legacy = r#"{"next_id":2,"tasks":[{"id":1,"title":"old","status":"open"}]}"#;
	assert_eq!(
		Store::from_json(legacy).unwrap().get(1).unwrap().priority,
		Priority::Normal
	);
}

#[test]
fn json_imports_are_transactional_with_explicit_conflict_modes() {
	let mut store = Store::default();
	store.create("existing").unwrap();
	let mut incoming = Store::default();
	incoming.create("incoming").unwrap();
	let data = incoming.export_json().unwrap();
	let before = store.clone();
	assert!(store.import_json(&data, ImportMode::Merge).is_err());
	assert_eq!(store, before);
	assert_eq!(store.import_json(&data, ImportMode::Remap).unwrap().imported, 1);
	assert_eq!(store.get(2).unwrap().title, "incoming");
	assert_eq!(store.import_json(&data, ImportMode::Replace).unwrap().imported, 1);
	assert_eq!(store, incoming);
	let invalid = data.replace("incoming", " ");
	assert!(store.import_json(&invalid, ImportMode::Replace).is_err());
	assert_eq!(store, incoming);
}

#[test]
fn csv_export_quotes_commas_newlines_and_unicode() {
	let mut store = Store::default();
	store.create("Buy café, tea\nand \"milk\"").unwrap();
	let data = store.export_csv().unwrap();
	let mut reader = csv::Reader::from_reader(data.as_bytes());
	assert_eq!(reader.headers().unwrap().get(0), Some("id"));
	let row = reader.records().next().unwrap().unwrap();
	assert_eq!(row.get(1), Some("Buy café, tea\nand \"milk\""));
	assert_eq!(Store::default().export_csv().unwrap().lines().count(), 1);
}

#[test]
fn csv_import_round_trips_metadata_and_rejects_bad_rows_atomically() {
	let mut source = Store::default();
	source.create("café, tea").unwrap();
	source.set_category(1, Some("home")).unwrap();
	source.set_priority(1, Priority::High).unwrap();
	source.set_due(1, Some(parse_date("2024-02-29").unwrap())).unwrap();
	source
		.set_reminder(1, Some("2024-02-28T18:00:00Z".parse().unwrap()))
		.unwrap();
	let csv = source.export_csv().unwrap();
	let mut restored = Store::default();
	restored.import_csv(&csv, ImportMode::Merge).unwrap();
	assert_eq!(restored, source);
	let before = restored.clone();
	assert!(restored
		.import_csv(&csv.replace("high", "unknown"), ImportMode::Replace)
		.is_err());
	assert!(restored.import_csv("title,id\na,1\n", ImportMode::Replace).is_err());
	assert_eq!(restored, before);
}

#[test]
fn statistics_count_open_overdue_and_grouped_tasks() {
	let mut store = Store::default();
	for title in ["a", "b", "c"] {
		store.create(title).unwrap();
	}
	store.set_category(1, Some("work")).unwrap();
	store.set_priority(1, Priority::High).unwrap();
	store.set_due(1, Some(parse_date("2024-06-01").unwrap())).unwrap();
	store.set_status(2, Status::Done).unwrap();
	let stats = store.statistics(parse_date("2024-06-03").unwrap());
	assert_eq!((stats.total, stats.open, stats.done, stats.overdue), (3, 2, 1, 1));
	assert_eq!(stats.uncategorized, 2);
	assert_eq!(stats.by_category["work"], 1);
	assert_eq!(stats.by_priority[&Priority::High], 1);
}

#[test]
fn clearing_completed_tasks_preserves_open_work_and_the_id_allocator() {
	let mut store = Store::default();
	store.create("open").unwrap();
	store.create("done").unwrap();
	store.set_status(2, Status::Done).unwrap();
	assert_eq!(store.clear_completed(), 1);
	assert_eq!(store.clear_completed(), 0);
	assert_eq!(store.tasks().len(), 1);
	assert_eq!(store.create("next").unwrap().id, 3);
}

#[test]
fn exclusive_store_locks_are_released_when_the_guard_is_dropped() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("nested/todos.json");
	let first = StoreLock::acquire(&path).unwrap();
	assert!(StoreLock::acquire(&path).is_err());
	drop(first);
	assert!(StoreLock::acquire(&path).is_ok());
}

#[cfg(unix)]
#[test]
fn data_symlinks_are_rejected_instead_of_splitting_the_store() {
	let dir = tempfile::tempdir().unwrap();
	let real = dir.path().join("real.json");
	let alias = dir.path().join("alias.json");
	let store = Store::default();
	store.save(&real).unwrap();
	std::os::unix::fs::symlink(&real, &alias).unwrap();
	assert!(Store::load(&alias).is_err());
	assert!(store.save(&alias).is_err());
	assert!(std::fs::symlink_metadata(&alias).unwrap().file_type().is_symlink());
}

#[test]
fn import_boundaries_reject_oversized_data_and_inconsistent_metadata() {
	let oversized = " ".repeat(MAX_DATA_BYTES + 1);
	assert!(Store::from_json(&oversized).is_err());
	let mut store = Store::default();
	assert!(store.import_csv(&oversized, ImportMode::Replace).is_err());
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("large.json");
	std::fs::File::create(&path)
		.unwrap()
		.set_len((MAX_DATA_BYTES + 1) as u64)
		.unwrap();
	assert!(Store::load(&path).is_err());
	let bad = r#"{"next_id":2,"tasks":[{"id":1,"title":"x","status":"open","reminder_dismissed":true}]}"#;
	assert!(Store::from_json(bad).is_err());
	assert!(parse_date("+12345-01-01").is_err());
}

#[test]
fn terminal_control_sequences_are_rejected_at_all_title_boundaries() {
	for title in ["bad\u{1b}]52;c;payload\u{7}", "bad\u{7}"] {
		let mut store = Store::default();
		assert!(store.create(title).is_err());
		store.create("safe").unwrap();
		assert!(store.rename(1, title).is_err());
		let mut document: serde_json::Value = serde_json::from_str(&store.export_json().unwrap()).unwrap();
		document["tasks"][0]["title"] = title.into();
		assert!(store.import_json(&document.to_string(), ImportMode::Replace).is_err());
		assert_eq!(store.get(1).unwrap().title, "safe");
	}
	assert!(validate_title("valid\nmultiline\ttext").is_ok());
}

#[test]
fn published_fixture_restores_metadata_and_deleted_id_history() {
	let data = include_str!("fixtures/version1.json");
	let mut store = Store::from_json(data).unwrap();
	assert_eq!(store.tasks().len(), 2);
	assert_eq!(
		store.query(&Query {
			status: Some(Status::Open),
			..Default::default()
		})[0]
			.title,
		"Review release"
	);
	assert_eq!(
		store.pending_reminders("2024-06-10T18:00:00Z".parse().unwrap()).len(),
		1
	);
	assert_eq!(store.create("next after deleted ids").unwrap().id, 5);
	assert_eq!(Store::from_json(&store.export_json().unwrap()).unwrap(), store);
	let mut invalid: serde_json::Value = serde_json::from_str(data).unwrap();
	invalid["tasks"][1]["id"] = 1.into();
	assert!(Store::from_json(&invalid.to_string()).is_err());
}
