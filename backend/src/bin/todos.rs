use chrono::{DateTime, NaiveDate, Utc};
use clap::{Parser, Subcommand};
use rusty_todos::local::{parse_date, Error, ImportMode, Priority, Query, Result, SortField, Status, Store, StoreLock};
use std::{
	io::Write,
	path::{Path, PathBuf},
	process::ExitCode,
};

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
	/// Summarize all tasks; overdue uses UTC today unless --as-of is supplied.
	Stats {
		#[arg(long, value_parser = parse_date)]
		as_of: Option<NaiveDate>,
	},
	/// Permanently remove completed tasks, preserving the id sequence.
	ClearDone {
		#[arg(long)]
		yes: bool,
	},
	/// Schedule, poll, acknowledge or clear reminders (no daemon).
	Reminder {
		#[command(subcommand)]
		command: ReminderAction,
	},
	/// Export a complete JSON backup or CSV exchange data to stdout/a new file.
	Export {
		#[arg(long, value_enum, default_value = "json")]
		format: DataFormat,
		#[arg(long)]
		output: Option<PathBuf>,
	},
	/// Import a backup; merge rejects id conflicts.
	Import {
		input: PathBuf,
		#[arg(long, value_enum, default_value = "json")]
		format: DataFormat,
		#[arg(long, value_enum, default_value = "merge")]
		mode: ImportMode,
		/// Required to replace the complete store.
		#[arg(long)]
		yes: bool,
	},
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
	let _lock = StoreLock::acquire(&cli.data)?;
	let mut store = Store::load(&cli.data)?;
	let (output, changed) = match cli.command {
		Action::Stats { as_of } => (
			serde_json::to_value(store.statistics(as_of.unwrap_or_else(|| Utc::now().date_naive())))?,
			false,
		),
		Action::ClearDone { yes } => {
			if !yes {
				return Err(Error::Invalid("clear-done requires --yes".into()));
			}
			(serde_json::json!({"deleted": store.clear_completed()}), true)
		}
		Action::Reminder { command } => match command {
			ReminderAction::List { now } => (
				serde_json::to_value(store.pending_reminders(now.unwrap_or_else(Utc::now)))?,
				false,
			),
			ReminderAction::Set { id, at } => (serde_json::to_value(store.set_reminder(id, Some(at))?)?, true),
			ReminderAction::Clear { id } => (serde_json::to_value(store.set_reminder(id, None)?)?, true),
			ReminderAction::Dismiss { id } => (serde_json::to_value(store.dismiss_reminder(id)?)?, true),
		},
		Action::Export { format, output } => {
			let data = match format {
				DataFormat::Json => store.export_json()?,
				DataFormat::Csv => store.export_csv()?,
			};
			if let Some(path) = output {
				write_export(&path, &data)?;
				(serde_json::json!({"exported": store.tasks().len()}), false)
			} else {
				std::io::stdout().lock().write_all(data.as_bytes())?;
				return Ok(());
			}
		}
		Action::Import {
			input,
			format,
			mode,
			yes,
		} => {
			if matches!(mode, ImportMode::Replace) && !yes {
				return Err(Error::Invalid("replace requires --yes; export a backup first".into()));
			}
			let data = rusty_todos::local::read_data(input)?;
			let summary = match format {
				DataFormat::Json => store.import_json(&data, mode)?,
				DataFormat::Csv => store.import_csv(&data, mode)?,
			};
			(serde_json::to_value(summary)?, true)
		}
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
	writeln!(std::io::stdout().lock(), "{}", serde_json::to_string_pretty(&output)?)?;
	Ok(())
}

fn main() -> ExitCode {
	match execute(Cli::parse()) {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			let _ = writeln!(std::io::stderr().lock(), "error: {error}");
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

#[derive(Clone, Copy, clap::ValueEnum)]
enum DataFormat {
	Json,
	Csv,
}

#[derive(Subcommand)]
enum ReminderAction {
	List {
		#[arg(long)]
		now: Option<DateTime<Utc>>,
	},
	Set {
		id: u64,
		at: DateTime<Utc>,
	},
	Clear {
		id: u64,
	},
	Dismiss {
		id: u64,
	},
}

fn write_export(path: &Path, data: &str) -> Result<()> {
	let parent = path
		.parent()
		.filter(|p| !p.as_os_str().is_empty())
		.unwrap_or(Path::new("."));
	std::fs::create_dir_all(parent)?;
	// NamedTempFile creates private files on Unix, independent of the process umask.
	let mut file = tempfile::NamedTempFile::new_in(parent)?;
	file.write_all(data.as_bytes())?;
	file.as_file().sync_all()?;
	file.persist_noclobber(path).map_err(|error| Error::Io(error.error))?;
	#[cfg(unix)]
	std::fs::File::open(parent)?.sync_all()?;
	Ok(())
}
