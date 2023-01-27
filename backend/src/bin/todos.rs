use clap::{Parser, Subcommand};
use rusty_todos::local::{Result, Status, Store};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(name = "rusty-todos", version, about = "Local-first todo manager")]
struct Cli {
	#[arg(long, env = "RUSTY_TODOS_FILE", default_value = "rusty-todos.json", global = true)]
	data: PathBuf,
	#[command(subcommand)]
	command: Action,
}

#[derive(Subcommand)]
enum Action {
	/// Add a todo; quote titles containing spaces.
	Add { title: String },
	/// List todos as JSON.
	List,
	/// Show one todo.
	Show { id: u64 },
	/// Rename a todo.
	Edit { id: u64, title: String },
	/// Delete one todo.
	Delete { id: u64 },
	/// Mark a todo complete.
	Done { id: u64 },
	/// Reopen a completed todo.
	Reopen { id: u64 },
}

fn execute(cli: Cli) -> Result<()> {
	let mut store = Store::load(&cli.data)?;
	let (output, changed) = match cli.command {
		Action::Add { title } => (serde_json::to_value(store.create(&title)?)?, true),
		Action::List => (serde_json::to_value(store.tasks())?, false),
		Action::Show { id } => (serde_json::to_value(store.get(id)?)?, false),
		Action::Edit { id, title } => (serde_json::to_value(store.rename(id, &title)?)?, true),
		Action::Delete { id } => (serde_json::to_value(store.delete(id)?)?, true),
		Action::Done { id } => (serde_json::to_value(store.set_status(id, Status::Done)?)?, true),
		Action::Reopen { id } => (serde_json::to_value(store.set_status(id, Status::Open)?)?, true),
	};
	if changed {
		store.save(&cli.data)?;
	}
	println!("{}", serde_json::to_string_pretty(&output)?);
	Ok(())
}

fn main() -> ExitCode {
	match execute(Cli::parse()) {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			eprintln!("error: {error}");
			ExitCode::FAILURE
		}
	}
}
