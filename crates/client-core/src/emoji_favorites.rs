use crate::State;
use model::{EmojiFavorite, MAX_EMOJI_FAVORITES};

#[derive(Default)]
pub struct Favorites {
	pub items: Vec<EmojiFavorite>,
	pub changed: bool,
	loaded: bool,
	edited: bool,
}

impl State {
	pub fn is_emoji_favorite(&self, favorite: &EmojiFavorite) -> bool {
		self.emoji_favorites
			.items
			.iter()
			.any(|known| known.same(favorite))
	}

	pub fn toggle_emoji_favorite(&mut self, favorite: EmojiFavorite) -> bool {
		if !favorite.valid() {
			return false;
		}
		if let Some(index) = self
			.emoji_favorites
			.items
			.iter()
			.position(|known| known.same(&favorite))
		{
			self.emoji_favorites.items.remove(index);
		} else {
			self.emoji_favorites.items.insert(0, favorite);
			self.emoji_favorites.items.truncate(MAX_EMOJI_FAVORITES);
			while !model::valid_emoji_favorites(&self.emoji_favorites.items) {
				self.emoji_favorites.items.pop();
			}
		}
		self.emoji_favorites.changed = true;
		self.emoji_favorites.edited = true;
		true
	}

	pub fn restore_emoji_favorites(&mut self, favorites: Vec<EmojiFavorite>) {
		if std::mem::replace(&mut self.emoji_favorites.loaded, true) || self.emoji_favorites.edited
		{
			return;
		}
		for favorite in favorites {
			if favorite.valid()
				&& self.emoji_favorites.items.len() < MAX_EMOJI_FAVORITES
				&& !self
					.emoji_favorites
					.items
					.iter()
					.any(|known| known.same(&favorite))
			{
				self.emoji_favorites.items.push(favorite);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use model::Id;

	fn custom(id: u64) -> EmojiFavorite {
		EmojiFavorite {
			id: Some(Id(id)),
			name: "wave".into(),
			animated: true,
		}
	}

	#[test]
	fn favorite_toggle_and_late_restore_keep_user_edits() {
		let mut state = State::default();
		assert!(state.toggle_emoji_favorite(custom(3)));
		state.restore_emoji_favorites(vec![custom(4)]);
		assert_eq!(state.emoji_favorites.items, vec![custom(3)]);
		assert!(state.toggle_emoji_favorite(custom(3)));
		assert!(state.emoji_favorites.items.is_empty());
	}

	#[test]
	fn restored_favorites_are_valid_deduplicated_and_bounded() {
		let mut state = State::default();
		state.restore_emoji_favorites(vec![custom(5), custom(5), custom(0)]);
		assert_eq!(state.emoji_favorites.items, vec![custom(5)]);
	}
}
