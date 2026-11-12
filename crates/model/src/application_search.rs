//! Read-only application catalog entries returned by the sampled Activities search route.
use crate::Id;

pub const MAX_RESULTS: usize = 50;
pub const MAX_CATALOG_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
	pub id: u16,
	pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityConfig {
	pub supported_platforms: Vec<String>,
	pub release_phase: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Application {
	pub id: Id,
	pub name: String,
	pub icon: Option<String>,
	pub description: String,
	pub verified: bool,
	pub discoverable: bool,
	pub monetized: bool,
	pub tags: Vec<String>,
	pub categories: Vec<Category>,
	pub activity: Option<ActivityConfig>,
}

impl Application {
	pub fn valid(&self) -> bool {
		self.id.0 != 0
			&& !self.name.trim().is_empty()
			&& self.name.len() <= 128
			&& self.description.len() <= 2048
			&& self.icon.as_ref().is_none_or(|icon| {
				!icon.is_empty()
					&& icon.len() <= 128
					&& icon.bytes().all(|byte| byte.is_ascii_hexdigit())
			}) && self.tags.len() <= 32
			&& self.tags.iter().all(|tag| tag.len() <= 64)
			&& self.categories.len() <= 16
			&& self
				.categories
				.iter()
				.all(|category| !category.name.trim().is_empty() && category.name.len() <= 64)
			&& self.activity.as_ref().is_none_or(|activity| {
				activity.supported_platforms.len() <= 8
					&& activity
						.supported_platforms
						.iter()
						.all(|platform| matches!(platform.as_str(), "web" | "ios" | "android"))
					&& activity
						.release_phase
						.as_ref()
						.is_none_or(|phase| phase.len() <= 32)
			})
	}

	pub fn bytes(&self) -> usize {
		std::mem::size_of::<Self>()
			+ self.name.capacity()
			+ self.icon.as_ref().map_or(0, String::capacity)
			+ self.description.capacity()
			+ self.tags.capacity() * std::mem::size_of::<String>()
			+ self.tags.iter().map(String::capacity).sum::<usize>()
			+ self.categories.capacity() * std::mem::size_of::<Category>()
			+ self
				.categories
				.iter()
				.map(|category| category.name.capacity())
				.sum::<usize>()
			+ self.activity.as_ref().map_or(0, |activity| {
				activity.supported_platforms.capacity() * std::mem::size_of::<String>()
					+ activity
						.supported_platforms
						.iter()
						.map(String::capacity)
						.sum::<usize>()
					+ activity.release_phase.as_ref().map_or(0, String::capacity)
			})
	}
}
