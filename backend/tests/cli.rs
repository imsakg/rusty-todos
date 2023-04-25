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
