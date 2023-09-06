use rusty_todos::local::*;

#[test]
fn titles_are_trimmed_and_blank_titles_rejected() {
	assert_eq!(validate_title("  first task  ").unwrap(), "first task");
	assert!(validate_title(" \n\t").is_err());
	assert!(validate_title(&"x".repeat(501)).is_err());
}
