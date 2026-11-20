//! One explicitly refreshed snapshot of the signed-in account's active quests.
use crate::{Command, auth::Failure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
	Enroll,
	ClaimReward,
}

const MAX_QUESTS: usize = 100;
const MAX_BYTES: usize = 256 * 1024;

#[derive(Default)]
pub struct Quests {
	pub items: Vec<model::Quest>,
	pub loading: bool,
	pub loaded: bool,
	pub error: Option<Failure>,
	pub action_pending: bool,
	pub action_error: Option<Failure>,
	request: u64,
	action_request: u64,
}

impl Quests {
	pub fn request(&mut self, refresh: bool) -> Option<Command> {
		if self.loading || self.action_pending || (self.loaded && !refresh) {
			return None;
		}
		self.request = self.request.wrapping_add(1).max(1);
		self.loading = true;
		self.error = None;
		Some(Command::Quests {
			request: self.request,
		})
	}

	pub fn request_action(&mut self, id: &str, action: Action) -> Option<Command> {
		if self.action_pending
			|| self.loading
			|| id.len() > 20
			|| !id.bytes().all(|c| c.is_ascii_digit())
		{
			return None;
		}
		let quest = self.items.iter().find(|quest| quest.id == id)?;
		match action {
			Action::Enroll if quest.enrolled || quest.completed || quest.claimed => return None,
			Action::ClaimReward if !quest.completed || quest.claimed => return None,
			_ => {}
		}
		self.action_request = self.action_request.wrapping_add(1).max(1);
		self.action_pending = true;
		self.action_error = None;
		Some(Command::QuestAction {
			request: self.action_request,
			quest_id: id.to_owned(),
			action,
		})
	}

	pub fn apply_action(
		&mut self,
		request: u64,
		id: &str,
		action: Action,
		result: Result<(), Failure>,
	) {
		if !self.action_pending || request != self.action_request {
			return;
		}
		self.action_pending = false;
		self.action_error = result.err();
		if self.action_error.is_some() {
			return;
		}
		if let Some(quest) = self.items.iter_mut().find(|quest| quest.id == id) {
			match action {
				Action::Enroll => quest.enrolled = true,
				Action::ClaimReward => quest.claimed = true,
			}
		}
		// Refresh status from Discord after a write rather than trusting local completion state.
		self.loaded = false;
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
			enrolled: false,
			completed: false,
			claimed: false,
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
		quests.apply(request + 1, Ok(vec![quest("1001")]));
		assert!(quests.loading);
		quests.apply(request, Ok(vec![quest("1002")]));
		assert_eq!(quests.items[0].id, "1002");
		assert!(quests.request(false).is_none());
		assert!(matches!(quests.request(true), Some(Command::Quests { .. })));
	}
	#[test]
	fn quest_actions_enforce_valid_ids_and_reconcile_completion_state() {
		let mut quests = Quests::default();
		let Some(Command::Quests { request }) = quests.request(false) else {
			panic!("first visit requests quests")
		};
		let mut item = quest("123456");
		item.completed = true;
		assert!(!item.valid(), "completed quest requires enrolled = true");
		item.enrolled = true;
		quests.apply(request, Ok(vec![item]));

		assert!(
			quests
				.request_action("bad-id", Action::ClaimReward)
				.is_none()
		);
		let Some(Command::QuestAction {
			request: action_req,
			..
		}) = quests.request_action("123456", Action::ClaimReward)
		else {
			panic!("valid completed quest can claim reward")
		};
		quests.apply_action(action_req, "123456", Action::ClaimReward, Ok(()));
		assert!(quests.items[0].claimed);
		assert!(!quests.loaded);
	}
}
