//! Explicit, bounded search state for the read-only embedded Activities catalog.
use crate::{Command, auth::Failure};

pub const MAX_QUERY_CHARS: usize = 128;

#[derive(Default)]
pub struct ActivitySearch {
	pub query: String,
	pub items: Vec<model::application_search::Application>,
	pub loading: bool,
	pub loaded: bool,
	pub error: Option<Failure>,
	request: u64,
}

impl ActivitySearch {
	pub fn request(&mut self, query: &str) -> Option<Command> {
		let query = query.trim();
		if self.loading
			|| query.is_empty()
			|| query.chars().count() > MAX_QUERY_CHARS
			|| query.chars().any(char::is_control)
		{
			return None;
		}
		self.query.clear();
		self.query.push_str(query);
		self.request = self.request.wrapping_add(1).max(1);
		self.loading = true;
		self.loaded = false;
		self.error = None;
		Some(Command::ActivitySearch {
			request: self.request,
			query: self.query.clone(),
		})
	}

	pub fn apply(
		&mut self,
		request: u64,
		result: Result<Vec<model::application_search::Application>, Failure>,
	) {
		if request != self.request || !self.loading {
			return;
		}
		self.loading = false;
		self.loaded = true;
		match result {
			Ok(items)
				if items.len() <= model::application_search::MAX_RESULTS
					&& items
						.iter()
						.all(model::application_search::Application::valid)
					&& items
						.iter()
						.map(model::application_search::Application::bytes)
						.sum::<usize>() <= model::application_search::MAX_CATALOG_BYTES =>
			{
				self.items = items;
			}
			Ok(_) => {
				self.items.clear();
				self.error = Some(Failure::Capacity);
			}
			Err(error) => self.error = Some(error),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn app(name: &str) -> model::application_search::Application {
		model::application_search::Application {
			id: model::Id(1),
			name: name.into(),
			icon: None,
			description: String::new(),
			verified: false,
			discoverable: true,
			monetized: false,
			tags: vec![],
			categories: vec![],
			activity: None,
		}
	}

	#[test]
	fn search_is_explicit_bounded_and_ignores_stale_results() {
		let mut search = ActivitySearch::default();
		assert!(search.request("  ").is_none());
		let Some(Command::ActivitySearch { request, query }) = search.request(" wordle ") else {
			panic!("valid explicit search should enqueue")
		};
		assert_eq!(query, "wordle");
		assert!(
			search.request("chess").is_none(),
			"only one search is in flight"
		);
		search.apply(request + 1, Ok(vec![app("stale")]));
		assert!(search.loading);
		search.apply(request, Ok(vec![app("Wordle")]));
		assert_eq!(search.items[0].name, "Wordle");
		assert!(search.request(&"x".repeat(MAX_QUERY_CHARS + 1)).is_none());
	}
}
