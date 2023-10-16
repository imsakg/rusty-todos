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
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn validate_title(title: &str) -> Result<String> {
	let title = title.trim();
	if title.is_empty() || title.chars().count() > 500 {
		return Err(Error::Invalid("title must contain 1–500 characters".into()));
	}
	Ok(title.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Store {
	next_id: u64,
	tasks: Vec<Task>,
}

impl Default for Store {
	fn default() -> Self {
		Self {
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
