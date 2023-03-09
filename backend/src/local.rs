use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid input: {0}")]
	Invalid(String),
	#[error("todo {0} was not found")]
	NotFound(u64),
	#[error("todo identifiers are exhausted")]
	IdExhausted,
	#[error(transparent)]
	Io(#[from] std::io::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
	#[error(transparent)]
	Csv(#[from] csv::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn validate_title(title: &str) -> Result<String> {
	let title = title.trim();
	if title.is_empty() || title.chars().count() > 500 {
		return Err(Error::Invalid("title must contain 1–500 characters".into()));
	}
	Ok(title.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Status {
	#[default]
	Open,
	Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
	pub id: u64,
	pub title: String,
	pub status: Status,
	#[serde(default)]
	pub category: Option<String>,
	#[serde(default)]
	pub priority: Priority,
	#[serde(default)]
	pub due: Option<NaiveDate>,
	#[serde(default)]
	pub reminder: Option<DateTime<Utc>>,
	#[serde(default)]
	pub reminder_dismissed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
	#[serde(default = "schema_version")]
	schema_version: u32,
	next_id: u64,
	tasks: Vec<Task>,
}

impl Default for Store {
	fn default() -> Self {
		Self {
			schema_version: 1,
			next_id: 1,
			tasks: Vec::new(),
		}
	}
}

impl Store {
	pub fn create(&mut self, title: &str) -> Result<Task> {
		let title = validate_title(title)?;
		let next = self.next_id.checked_add(1).ok_or(Error::IdExhausted)?;
		let task = Task {
			id: self.next_id,
			title,
			status: Status::Open,
			category: None,
			priority: Priority::Normal,
			due: None,
			reminder: None,
			reminder_dismissed: false,
		};
		self.tasks.push(task.clone());
		self.next_id = next;
		Ok(task)
	}
}

impl Store {
	pub fn tasks(&self) -> &[Task] {
		&self.tasks
	}

	pub fn get(&self, id: u64) -> Result<&Task> {
		self.tasks.iter().find(|task| task.id == id).ok_or(Error::NotFound(id))
	}

	fn get_mut(&mut self, id: u64) -> Result<&mut Task> {
		self.tasks
			.iter_mut()
			.find(|task| task.id == id)
			.ok_or(Error::NotFound(id))
	}
}

impl Store {
	pub fn rename(&mut self, id: u64, title: &str) -> Result<Task> {
		let title = validate_title(title)?;
		let task = self.get_mut(id)?;
		task.title = title;
		Ok(task.clone())
	}
}

impl Store {
	pub fn delete(&mut self, id: u64) -> Result<Task> {
		let index = self
			.tasks
			.iter()
			.position(|task| task.id == id)
			.ok_or(Error::NotFound(id))?;
		Ok(self.tasks.remove(index))
	}
}

impl Store {
	pub fn set_status(&mut self, id: u64, status: Status) -> Result<Task> {
		let task = self.get_mut(id)?;
		task.status = status;
		Ok(task.clone())
	}
}

impl Store {
	fn validate(&self) -> Result<()> {
		if self.schema_version != 1 {
			return Err(Error::Invalid(format!(
				"unsupported schema version {}",
				self.schema_version
			)));
		}
		let mut ids = std::collections::HashSet::new();
		for task in &self.tasks {
			if let Some(category) = &task.category {
				if validate_category(category)? != *category {
					return Err(Error::Invalid("stored categories must be trimmed".into()));
				}
			}
			if task.id == 0 || task.id >= self.next_id || !ids.insert(task.id) {
				return Err(Error::Invalid("duplicate, zero or out-of-range todo id".into()));
			}
			if validate_title(&task.title)? != task.title {
				return Err(Error::Invalid("stored titles must be trimmed".into()));
			}
		}
		if self.next_id == 0 {
			return Err(Error::Invalid("next_id must be positive".into()));
		}
		Ok(())
	}

	pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
		match std::fs::File::open(path) {
			Ok(file) => {
				let store: Self = serde_json::from_reader(file)?;
				store.validate()?;
				Ok(store)
			}
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
			Err(error) => Err(error.into()),
		}
	}

	pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
		self.validate()?;
		let data = serde_json::to_vec_pretty(self)?;
		use std::io::Write;
		let path = path.as_ref();
		let parent = path
			.parent()
			.filter(|p| !p.as_os_str().is_empty())
			.unwrap_or(std::path::Path::new("."));
		std::fs::create_dir_all(parent)?;
		let mut file = tempfile::NamedTempFile::new_in(parent)?;
		file.write_all(&data)?;
		file.write_all(b"\n")?;
		file.as_file().sync_all()?;
		file.persist(path).map_err(|error| Error::Io(error.error))?;
		#[cfg(unix)]
		std::fs::File::open(parent)?.sync_all()?;
		Ok(())
	}
}

pub fn validate_category(name: &str) -> Result<String> {
	let name = name.trim();
	if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
		return Err(Error::Invalid("category must contain 1–64 printable characters".into()));
	}
	Ok(name.to_owned())
}

impl Store {
	pub fn set_category(&mut self, id: u64, name: Option<&str>) -> Result<Task> {
		let category = name.map(validate_category).transpose()?;
		let task = self.get_mut(id)?;
		task.category = category;
		Ok(task.clone())
	}
}

impl Store {
	pub fn categories(&self) -> Vec<&str> {
		self.tasks
			.iter()
			.filter_map(|task| task.category.as_deref())
			.collect::<std::collections::BTreeSet<_>>()
			.into_iter()
			.collect()
	}

	pub fn rename_category(&mut self, from: &str, to: &str) -> Result<usize> {
		let to = validate_category(to)?;
		let count = self
			.tasks
			.iter()
			.filter(|task| task.category.as_deref() == Some(from))
			.count();
		if count == 0 {
			return Err(Error::Invalid(format!("category {from:?} was not found")));
		}
		for task in &mut self.tasks {
			if task.category.as_deref() == Some(from) {
				task.category = Some(to.clone());
			}
		}
		Ok(count)
	}

	pub fn remove_category(&mut self, name: &str) -> Result<usize> {
		let count = self
			.tasks
			.iter()
			.filter(|task| task.category.as_deref() == Some(name))
			.count();
		if count == 0 {
			return Err(Error::Invalid(format!("category {name:?} was not found")));
		}
		for task in &mut self.tasks {
			if task.category.as_deref() == Some(name) {
				task.category = None;
			}
		}
		Ok(count)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
	Low,
	#[default]
	Normal,
	High,
	Urgent,
}

impl Store {
	pub fn set_priority(&mut self, id: u64, priority: Priority) -> Result<Task> {
		let task = self.get_mut(id)?;
		task.priority = priority;
		Ok(task.clone())
	}
}

pub fn parse_date(value: &str) -> Result<NaiveDate> {
	let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
		.map_err(|_| Error::Invalid("date must be a real YYYY-MM-DD date".into()))?;
	if date.format("%Y-%m-%d").to_string() != value {
		return Err(Error::Invalid("date must use YYYY-MM-DD".into()));
	}
	Ok(date)
}

impl Store {
	pub fn set_due(&mut self, id: u64, due: Option<NaiveDate>) -> Result<Task> {
		let task = self.get_mut(id)?;
		task.due = due;
		Ok(task.clone())
	}
}

impl Store {
	pub fn set_reminder(&mut self, id: u64, reminder: Option<DateTime<Utc>>) -> Result<Task> {
		let task = self.get_mut(id)?;
		task.reminder = reminder;
		task.reminder_dismissed = false;
		Ok(task.clone())
	}

	pub fn dismiss_reminder(&mut self, id: u64) -> Result<Task> {
		let task = self.get_mut(id)?;
		if task.reminder.is_none() {
			return Err(Error::Invalid("todo has no reminder".into()));
		}
		task.reminder_dismissed = true;
		Ok(task.clone())
	}

	pub fn pending_reminders(&self, now: DateTime<Utc>) -> Vec<&Task> {
		let mut tasks: Vec<_> = self
			.tasks
			.iter()
			.filter(|task| {
				task.status == Status::Open && !task.reminder_dismissed && task.reminder.is_some_and(|at| at <= now)
			})
			.collect();
		tasks.sort_by_key(|task| (task.reminder, task.id));
		tasks
	}
}

#[derive(Debug, Clone, Default)]
pub struct Query {
	pub status: Option<Status>,
	pub category: Option<String>,
	pub uncategorized: bool,
	pub priority: Option<Priority>,
	pub search: Option<String>,
	pub due_before: Option<NaiveDate>,
	pub overdue_as_of: Option<NaiveDate>,
	pub sort: SortField,
	pub descending: bool,
	pub offset: usize,
	pub limit: Option<usize>,
}

impl Store {
	pub fn query(&self, query: &Query) -> Vec<&Task> {
		let search = query.search.as_ref().map(|value| value.to_lowercase());
		let mut tasks: Vec<_> = self
			.tasks
			.iter()
			.filter(|task| {
				search
					.as_ref()
					.is_none_or(|text| task.title.to_lowercase().contains(text))
					&& query
						.due_before
						.is_none_or(|date| task.due.is_some_and(|due| due <= date))
					&& query
						.overdue_as_of
						.is_none_or(|date| task.status == Status::Open && task.due.is_some_and(|due| due < date))
					&& query.status.is_none_or(|status| task.status == status)
					&& query
						.category
						.as_ref()
						.is_none_or(|category| task.category.as_ref() == Some(category))
					&& (!query.uncategorized || task.category.is_none())
					&& query.priority.is_none_or(|priority| task.priority == priority)
			})
			.collect();
		tasks.sort_by(|a, b| {
			use std::cmp::Ordering;
			let order = match query.sort {
				SortField::Id => a.id.cmp(&b.id),
				SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
				SortField::Priority => a.priority.cmp(&b.priority),
				SortField::Due => match (a.due, b.due) {
					(None, Some(_)) => return Ordering::Greater,
					(Some(_), None) => return Ordering::Less,
					(a, b) => a.cmp(&b),
				},
			};
			let order = if query.descending { order.reverse() } else { order };
			order.then(a.id.cmp(&b.id))
		});
		tasks
			.into_iter()
			.skip(query.offset)
			.take(query.limit.unwrap_or(usize::MAX))
			.collect()
	}
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum SortField {
	#[default]
	Id,
	Title,
	Priority,
	Due,
}

fn schema_version() -> u32 {
	1
}

impl Store {
	pub fn from_json(data: &str) -> Result<Self> {
		let store: Self = serde_json::from_str(data)?;
		store.validate()?;
		Ok(store)
	}

	pub fn export_json(&self) -> Result<String> {
		self.validate()?;
		Ok(format!("{}\n", serde_json::to_string_pretty(self)?))
	}
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ImportMode {
	Merge,
	Replace,
	Remap,
}

#[derive(Debug, Serialize)]
pub struct ImportSummary {
	pub imported: usize,
}

impl Store {
	pub fn import_json(&mut self, data: &str, mode: ImportMode) -> Result<ImportSummary> {
		self.import_store(Self::from_json(data)?, mode)
	}

	fn import_store(&mut self, incoming: Self, mode: ImportMode) -> Result<ImportSummary> {
		incoming.validate()?;
		let imported = incoming.tasks.len();
		let mut candidate = self.clone();
		match mode {
			ImportMode::Replace => candidate = incoming,
			ImportMode::Merge => {
				if incoming
					.tasks
					.iter()
					.any(|task| candidate.tasks.iter().any(|existing| task.id == existing.id))
				{
					return Err(Error::Invalid(
						"import contains an existing id; choose remap or replace".into(),
					));
				}
				candidate.next_id = candidate.next_id.max(incoming.next_id);
				candidate.tasks.extend(incoming.tasks);
			}
			ImportMode::Remap => {
				for mut task in incoming.tasks {
					let next = candidate.next_id.checked_add(1).ok_or(Error::IdExhausted)?;
					task.id = candidate.next_id;
					candidate.next_id = next;
					candidate.tasks.push(task);
				}
			}
		}
		candidate.validate()?;
		*self = candidate;
		Ok(ImportSummary { imported })
	}
}

pub const CSV_COLUMNS: [&str; 8] = [
	"id",
	"title",
	"status",
	"category",
	"priority",
	"due",
	"reminder",
	"reminder_dismissed",
];

impl Store {
	pub fn export_csv(&self) -> Result<String> {
		self.validate()?;
		let mut writer = csv::WriterBuilder::new().has_headers(false).from_writer(Vec::new());
		writer.write_record(CSV_COLUMNS)?;
		for task in &self.tasks {
			writer.serialize(task)?;
		}
		let bytes = writer.into_inner().map_err(|error| Error::Io(error.into_error()))?;
		String::from_utf8(bytes).map_err(|_| Error::Invalid("CSV output is not UTF-8".into()))
	}
}

impl Store {
	pub fn import_csv(&mut self, data: &str, mode: ImportMode) -> Result<ImportSummary> {
		let mut reader = csv::ReaderBuilder::new().from_reader(data.as_bytes());
		if !reader.headers()?.iter().eq(CSV_COLUMNS) {
			return Err(Error::Invalid("CSV header does not match the exported schema".into()));
		}
		let tasks: Vec<Task> = reader.deserialize().collect::<std::result::Result<_, _>>()?;
		let next_id = tasks
			.iter()
			.map(|task| task.id)
			.max()
			.unwrap_or(0)
			.checked_add(1)
			.ok_or(Error::IdExhausted)?;
		let incoming = Self {
			schema_version: 1,
			next_id,
			tasks,
		};
		self.import_store(incoming, mode)
	}
}

#[derive(Debug, Serialize)]
pub struct Statistics {
	pub total: usize,
	pub open: usize,
	pub done: usize,
	pub overdue: usize,
	pub uncategorized: usize,
	pub by_category: std::collections::BTreeMap<String, usize>,
	pub by_priority: std::collections::BTreeMap<Priority, usize>,
}

impl Store {
	pub fn statistics(&self, as_of: NaiveDate) -> Statistics {
		let mut stats = Statistics {
			total: self.tasks.len(),
			open: 0,
			done: 0,
			overdue: 0,
			uncategorized: 0,
			by_category: Default::default(),
			by_priority: Default::default(),
		};
		for task in &self.tasks {
			match task.status {
				Status::Open => {
					stats.open += 1;
					if task.due.is_some_and(|due| due < as_of) {
						stats.overdue += 1;
					}
				}
				Status::Done => stats.done += 1,
			}
			if let Some(category) = &task.category {
				*stats.by_category.entry(category.clone()).or_default() += 1;
			} else {
				stats.uncategorized += 1;
			}
			*stats.by_priority.entry(task.priority).or_default() += 1;
		}
		stats
	}
}

impl Store {
	pub fn clear_completed(&mut self) -> usize {
		let before = self.tasks.len();
		self.tasks.retain(|task| task.status != Status::Done);
		before - self.tasks.len()
	}
}
