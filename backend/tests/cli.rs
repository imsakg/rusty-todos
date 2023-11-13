use std::path::Path;
use std::process::{Command, Output};

fn cli(path: &Path, args: &[&str]) -> Output {
	Command::new(env!("CARGO_BIN_EXE_rusty-todos"))
		.arg("--data")
		.arg(path)
		.args(args)
		.output()
		.unwrap()
}

#[test]
fn add_list_and_show_work_across_processes() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	let added = cli(&path, &["add", "hello"]);
	assert!(added.status.success(), "{}", String::from_utf8_lossy(&added.stderr));
	let shown = cli(&path, &["show", "1"]);
	assert!(shown.status.success());
	assert_eq!(
		serde_json::from_slice::<serde_json::Value>(&shown.stdout).unwrap()["title"],
		"hello"
	);
	let listed = cli(&path, &["list"]);
	assert_eq!(
		serde_json::from_slice::<Vec<serde_json::Value>>(&listed.stdout)
			.unwrap()
			.len(),
		1
	);
	assert!(!cli(&path, &["show", "999"]).status.success());
}

#[test]
fn edits_completion_and_deletion_persist() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	assert!(cli(&path, &["add", "before"]).status.success());
	for args in [&["edit", "1", "after"][..], &["done", "1"], &["reopen", "1"]] {
		assert!(cli(&path, args).status.success());
	}
	assert!(cli(&path, &["delete", "1"]).status.success());
	assert!(!cli(&path, &["show", "1"]).status.success());
}

#[test]
fn categories_can_be_managed_from_the_cli() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	cli(&path, &["add", "categorized"]);
	assert!(cli(&path, &["category", "assign", "1", "work"]).status.success());
	assert!(cli(&path, &["category", "rename", "work", "office"]).status.success());
	let output = cli(&path, &["category", "list"]);
	assert_eq!(
		serde_json::from_slice::<Vec<String>>(&output.stdout).unwrap(),
		vec!["office"]
	);
	assert!(cli(&path, &["category", "clear", "1"]).status.success());
	assert!(!cli(&path, &["category", "remove", "missing"]).status.success());
}

#[test]
fn priority_and_due_updates_are_validated_before_saving() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	cli(&path, &["add", "scheduled"]);
	assert!(cli(&path, &["set", "1", "--priority", "urgent", "--due", "2024-02-29"])
		.status
		.success());
	let before = std::fs::read(&path).unwrap();
	assert!(!cli(&path, &["set", "1", "--priority", "low", "--due", "2023-02-29"])
		.status
		.success());
	assert_eq!(std::fs::read(&path).unwrap(), before);
	assert!(cli(&path, &["set", "1", "--clear-due"]).status.success());
	assert!(!cli(&path, &["set", "1"]).status.success());
}

#[test]
fn list_filters_sorts_and_pages_the_stored_tasks() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	cli(&path, &["add", "Report Z"]);
	cli(&path, &["add", "report A"]);
	cli(&path, &["set", "2", "--due", "2024-06-01", "--priority", "high"]);
	let output = cli(
		&path,
		&[
			"list", "--status", "open", "--search", "REPORT", "--sort", "title", "--limit", "1",
		],
	);
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	let rows: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
	assert_eq!(rows[0]["id"], 2);
	let overdue = cli(
		&path,
		&["list", "--overdue", "--as-of", "2024-06-03", "--priority", "high"],
	);
	assert_eq!(
		serde_json::from_slice::<Vec<serde_json::Value>>(&overdue.stdout)
			.unwrap()
			.len(),
		1
	);
	assert!(!cli(&path, &["list", "--category", "work", "--uncategorized"])
		.status
		.success());
}

#[test]
fn backup_commands_round_trip_without_silent_overwrites() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("source.json");
	let target = dir.path().join("restored.json");
	let backup = dir.path().join("backup.json");
	cli(&path, &["add", "backup me"]);
	let exported = cli(&path, &["export"]);
	assert!(exported.status.success());
	std::fs::write(&backup, &exported.stdout).unwrap();
	assert!(cli(&target, &["import", backup.to_str().unwrap()]).status.success());
	assert!(!cli(&target, &["import", backup.to_str().unwrap()]).status.success());
	assert!(
		!cli(&target, &["import", backup.to_str().unwrap(), "--mode", "replace"])
			.status
			.success()
	);
	assert!(cli(
		&target,
		&["import", backup.to_str().unwrap(), "--mode", "replace", "--yes"]
	)
	.status
	.success());
	assert!(!cli(&path, &["export", "--output", backup.to_str().unwrap()])
		.status
		.success());
	let csv = dir.path().join("backup.csv");
	assert!(
		cli(&path, &["export", "--format", "csv", "--output", csv.to_str().unwrap()])
			.status
			.success()
	);
	let other = dir.path().join("other.json");
	assert!(cli(&other, &["import", csv.to_str().unwrap(), "--format", "csv"])
		.status
		.success());
}

#[test]
fn reminder_commands_use_explicit_time_and_do_not_redeliver_dismissed_items() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	cli(&path, &["add", "call"]);
	assert!(cli(&path, &["reminder", "set", "1", "2024-06-03T21:00:00+03:00"])
		.status
		.success());
	let pending = cli(&path, &["reminder", "list", "--now", "2024-06-03T18:00:00Z"]);
	assert_eq!(
		serde_json::from_slice::<Vec<serde_json::Value>>(&pending.stdout)
			.unwrap()
			.len(),
		1
	);
	assert!(cli(&path, &["reminder", "dismiss", "1"]).status.success());
	let pending = cli(&path, &["reminder", "list", "--now", "2024-06-03T18:00:00Z"]);
	assert!(serde_json::from_slice::<Vec<serde_json::Value>>(&pending.stdout)
		.unwrap()
		.is_empty());
	assert!(cli(&path, &["reminder", "clear", "1"]).status.success());
}

#[test]
fn statistics_and_confirmed_cleanup_are_available() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	cli(&path, &["add", "finish"]);
	cli(&path, &["done", "1"]);
	let stats = cli(&path, &["stats", "--as-of", "2024-06-03"]);
	assert!(stats.status.success());
	assert_eq!(
		serde_json::from_slice::<serde_json::Value>(&stats.stdout).unwrap()["done"],
		1
	);
	assert!(!cli(&path, &["clear-done"]).status.success());
	assert!(cli(&path, &["clear-done", "--yes"]).status.success());
	assert!(
		serde_json::from_slice::<Vec<serde_json::Value>>(&cli(&path, &["list"]).stdout)
			.unwrap()
			.is_empty()
	);
}

#[test]
fn held_locks_and_corrupt_files_fail_without_losing_data() {
	use rusty_todos::local::StoreLock;
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	cli(&path, &["add", "keep"]);
	let before = std::fs::read(&path).unwrap();
	let lock = StoreLock::acquire(&path).unwrap();
	let output = cli(&path, &["add", "blocked"]);
	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("locked"));
	assert_eq!(std::fs::read(&path).unwrap(), before);
	drop(lock);
	std::fs::write(&path, "broken JSON").unwrap();
	assert!(!cli(&path, &["add", "do not replace"]).status.success());
	assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken JSON");
}

#[test]
fn parallel_processes_retry_contention_without_losing_successful_writes() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("todos.json");
	let handles: Vec<_> = (0..8)
		.map(|n| {
			let path = path.clone();
			std::thread::spawn(move || {
				for _ in 0..100 {
					let output = cli(&path, &["add", &format!("parallel {n}")]);
					if output.status.success() {
						return;
					}
					assert!(String::from_utf8_lossy(&output.stderr).contains("locked"));
					std::thread::sleep(std::time::Duration::from_millis(10));
				}
				panic!("store remained locked");
			})
		})
		.collect();
	for handle in handles {
		handle.join().unwrap();
	}
	let store = rusty_todos::local::Store::load(&path).unwrap();
	assert_eq!(store.tasks().len(), 8);
	let ids: std::collections::BTreeSet<_> = store.tasks().iter().map(|task| task.id).collect();
	assert_eq!(ids, (1..=8).collect());
}

#[test]
fn environment_config_and_read_only_commands_do_not_create_a_store() {
	let dir = tempfile::tempdir().unwrap();
	let path = dir.path().join("custom.json");
	let output = Command::new(env!("CARGO_BIN_EXE_rusty-todos"))
		.env("RUSTY_TODOS_FILE", &path)
		.arg("list")
		.output()
		.unwrap();
	assert!(output.status.success());
	assert!(!path.exists()); // only an advisory lock sidecar is created
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[]");
	let output = cli(&path, &["add", " "]);
	assert!(!output.status.success());
	assert!(output.stdout.is_empty());
	assert!(!path.exists());
}
