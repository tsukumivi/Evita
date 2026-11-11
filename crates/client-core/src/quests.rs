//! One explicitly refreshed snapshot of the signed-in account's active quests.
use crate::{Command, auth::Failure};

const MAX_QUESTS: usize = 100;
const MAX_BYTES: usize = 256 * 1024;

#[derive(Default)]
pub struct Quests {
	pub items: Vec<model::Quest>,
	pub loading: bool,
	pub loaded: bool,
	pub error: Option<Failure>,
	request: u64,
}

impl Quests {
	pub fn request(&mut self, refresh: bool) -> Option<Command> {
		if self.loading || (self.loaded && !refresh) {
			return None;
		}
		self.request = self.request.wrapping_add(1).max(1);
		self.loading = true;
		self.error = None;
		Some(Command::Quests {
			request: self.request,
		})
	}

	pub fn apply(&mut self, request: u64, result: Result<Vec<model::Quest>, Failure>) {
		if request != self.request || !self.loading {
			return;
		}
		self.loading = false;
		self.loaded = true;
		match result {
			Ok(items)
				if items.len() <= MAX_QUESTS
					&& items.iter().all(model::Quest::valid)
					&& items.iter().map(model::Quest::bytes).sum::<usize>() <= MAX_BYTES =>
			{
				self.items = items;
				self.error = None;
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

	fn quest(id: &str) -> model::Quest {
		model::Quest {
			id: id.into(),
			title: "Synthetic quest".into(),
			description: "Synthetic objective".into(),
			progress: String::new(),
			progress_percent: None,
			image: None,
		}
	}

	#[test]
	fn refresh_is_explicit_and_stale_responses_are_ignored() {
		let mut quests = Quests::default();
		let Some(Command::Quests { request }) = quests.request(false) else {
			panic!("first visit requests quests")
		};
		assert!(quests.request(false).is_none());
		quests.apply(request + 1, Ok(vec![quest("stale")]));
		assert!(quests.loading);
		quests.apply(request, Ok(vec![quest("current")]));
		assert_eq!(quests.items[0].id, "current");
		assert!(quests.request(false).is_none());
		assert!(matches!(quests.request(true), Some(Command::Quests { .. })));
	}
}
