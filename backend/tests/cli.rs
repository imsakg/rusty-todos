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
