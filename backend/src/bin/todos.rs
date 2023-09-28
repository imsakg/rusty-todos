use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use rusty_todos::local::{parse_date, Error, Priority, Query, Result, SortField, Status, Store};
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
	/// Change optional task metadata atomically.
	Set {
		id: u64,
		#[arg(long, value_enum)]
		priority: Option<Priority>,
		#[arg(long, value_parser = parse_date, conflicts_with = "clear_due")]
		due: Option<NaiveDate>,
		#[arg(long)]
		clear_due: bool,
	},
	/// Add a todo; quote titles containing spaces.
	Add { title: String },
	/// List todos as JSON.
	List(ListArgs),
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
	/// Assign, rename, list or remove categories.
	Category {
		#[command(subcommand)]
		command: CategoryAction,
	},
}

#[derive(Subcommand)]
enum CategoryAction {
	List,
	Assign {
		id: u64,
		name: String,
	},
	Clear {
		id: u64,
	},
	Rename {
		from: String,
		to: String,
	},
	/// Remove a category from every member; keep the tasks.
	Remove {
		name: String,
	},
}

fn execute(cli: Cli) -> Result<()> {
	let mut store = Store::load(&cli.data)?;
	let (output, changed) = match cli.command {
		Action::Set {
			id,
			priority,
			due,
			clear_due,
		} => {
			if priority.is_none() && due.is_none() && !clear_due {
				return Err(Error::Invalid("set requires at least one change".into()));
			}
			if let Some(priority) = priority {
				store.set_priority(id, priority)?;
			}
			if due.is_some() || clear_due {
				store.set_due(id, due)?;
			}
			(serde_json::to_value(store.get(id)?)?, true)
		}
		Action::Category { command } => match command {
			CategoryAction::List => (serde_json::to_value(store.categories())?, false),
			CategoryAction::Assign { id, name } => (serde_json::to_value(store.set_category(id, Some(&name))?)?, true),
			CategoryAction::Clear { id } => (serde_json::to_value(store.set_category(id, None)?)?, true),
			CategoryAction::Rename { from, to } => {
				(serde_json::json!({"updated": store.rename_category(&from, &to)?}), true)
			}
			CategoryAction::Remove { name } => (serde_json::json!({"updated": store.remove_category(&name)?}), true),
		},
		Action::Add { title } => (serde_json::to_value(store.create(&title)?)?, true),
		Action::List(args) => (serde_json::to_value(store.query(&args.query()))?, false),
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

#[derive(clap::Args)]
struct ListArgs {
	#[arg(long, value_enum)]
	status: Option<Status>,
	#[arg(long, conflicts_with = "uncategorized")]
	category: Option<String>,
	#[arg(long)]
	uncategorized: bool,
	#[arg(long, value_enum)]
	priority: Option<Priority>,
	#[arg(long)]
	search: Option<String>,
	#[arg(long, value_parser = parse_date)]
	due_before: Option<NaiveDate>,
	#[arg(long)]
	overdue: bool,
	#[arg(long, requires = "overdue", value_parser = parse_date)]
	as_of: Option<NaiveDate>,
	#[arg(long, value_enum, default_value = "id")]
	sort: SortField,
	#[arg(long)]
	desc: bool,
	#[arg(long, default_value_t = 0)]
	offset: usize,
	#[arg(long)]
	limit: Option<usize>,
}

impl ListArgs {
	fn query(self) -> Query {
		Query {
			status: self.status,
			category: self.category,
			uncategorized: self.uncategorized,
			priority: self.priority,
			search: self.search,
			due_before: self.due_before,
			overdue_as_of: self
				.overdue
				.then(|| self.as_of.unwrap_or_else(|| Utc::now().date_naive())),
			sort: self.sort,
			descending: self.desc,
			offset: self.offset,
			limit: self.limit,
		}
	}
}
