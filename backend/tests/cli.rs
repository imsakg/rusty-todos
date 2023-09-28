use std::process::{Command, Output};
use std::path::Path;

fn cli(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rusty-todos"))
        .arg("--data").arg(path).args(args).output().unwrap()
}

#[test]
fn add_list_and_show_work_across_processes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todos.json");
    let added = cli(&path, &["add", "hello"]);
    assert!(added.status.success(), "{}", String::from_utf8_lossy(&added.stderr));
    let shown = cli(&path, &["show", "1"]);
    assert!(shown.status.success());
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&shown.stdout).unwrap()["title"], "hello");
    let listed = cli(&path, &["list"]);
    assert_eq!(serde_json::from_slice::<Vec<serde_json::Value>>(&listed.stdout).unwrap().len(), 1);
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
    let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("todos.json");
    cli(&path, &["add", "categorized"]);
    assert!(cli(&path, &["category", "assign", "1", "work"]).status.success());
    assert!(cli(&path, &["category", "rename", "work", "office"]).status.success());
    let output = cli(&path, &["category", "list"]);
    assert_eq!(serde_json::from_slice::<Vec<String>>(&output.stdout).unwrap(), vec!["office"]);
    assert!(cli(&path, &["category", "clear", "1"]).status.success());
    assert!(!cli(&path, &["category", "remove", "missing"]).status.success());
}

#[test]
fn priority_and_due_updates_are_validated_before_saving() {
    let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("todos.json");
    cli(&path, &["add", "scheduled"]);
    assert!(cli(&path, &["set", "1", "--priority", "urgent", "--due", "2024-02-29"]).status.success());
    let before = std::fs::read(&path).unwrap();
    assert!(!cli(&path, &["set", "1", "--priority", "low", "--due", "2023-02-29"]).status.success());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(cli(&path, &["set", "1", "--clear-due"]).status.success());
    assert!(!cli(&path, &["set", "1"]).status.success());
}

#[test]
fn list_filters_sorts_and_pages_the_stored_tasks() {
    let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("todos.json");
    cli(&path, &["add", "Report Z"]); cli(&path, &["add", "report A"]);
    cli(&path, &["set", "2", "--due", "2024-06-01", "--priority", "high"]);
    let output = cli(&path, &["list", "--status", "open", "--search", "REPORT", "--sort", "title", "--limit", "1"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rows[0]["id"], 2);
    let overdue = cli(&path, &["list", "--overdue", "--as-of", "2024-06-03", "--priority", "high"]);
    assert_eq!(serde_json::from_slice::<Vec<serde_json::Value>>(&overdue.stdout).unwrap().len(), 1);
    assert!(!cli(&path, &["list", "--category", "work", "--uncategorized"]).status.success());
}
