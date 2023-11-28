use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid input: {0}")]
	Invalid(String),
	#[error("todo {0} was not found")]
	NotFound(u64),
	#[error("todo identifiers are exhausted")]
	IdExhausted,
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
