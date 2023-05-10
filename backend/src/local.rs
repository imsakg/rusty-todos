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
