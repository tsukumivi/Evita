use crate::Id;

/// A user's locally saved emoji picker entry. Unicode names contain the actual sequence;
/// custom names use Discord's catalog slug and ID so artwork can be resolved from session data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmojiFavorite {
	pub id: Option<Id>,
	pub name: String,
	pub animated: bool,
}

pub const MAX_EMOJI_FAVORITES: usize = 100;
const MAX_FAVORITE_BYTES: usize = 16 * 1024;

impl EmojiFavorite {
	pub fn valid(&self) -> bool {
		if self.name.is_empty() || self.name.len() > 128 || self.name.chars().any(char::is_control)
		{
			return false;
		}
		match self.id {
			Some(id) => {
				id.0 != 0
					&& (2..=32).contains(&self.name.len())
					&& self
						.name
						.bytes()
						.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
			}
			None => !self.animated,
		}
	}

	pub fn same(&self, other: &Self) -> bool {
		match (self.id, other.id) {
			(Some(id), Some(other)) => id == other,
			(None, None) => self.name == other.name,
			_ => false,
		}
	}

	pub fn markup(&self) -> String {
		match self.id {
			Some(id) => format!(
				"<{}:{}:{}>",
				if self.animated { "a" } else { "" },
				self.name,
				id
			),
			None => self.name.clone(),
		}
	}

	pub fn heap_bytes(&self) -> usize {
		self.name.capacity()
	}
}

pub fn emoji_favorite_bytes(items: &[EmojiFavorite]) -> usize {
	std::mem::size_of_val(items) + items.iter().map(EmojiFavorite::heap_bytes).sum::<usize>()
}

pub fn valid_emoji_favorites(items: &[EmojiFavorite]) -> bool {
	items.len() <= MAX_EMOJI_FAVORITES
		&& emoji_favorite_bytes(items) <= MAX_FAVORITE_BYTES
		&& items.iter().enumerate().all(|(index, item)| {
			item.valid() && !items[..index].iter().any(|known| known.same(item))
		})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn favorites_distinguish_unicode_and_custom_ids_and_enforce_bounds() {
		let unicode = EmojiFavorite {
			id: None,
			name: "🫶".into(),
			animated: false,
		};
		let custom = EmojiFavorite {
			id: Some(Id(7)),
			name: "wave".into(),
			animated: true,
		};
		assert!(unicode.valid());
		assert_eq!(custom.markup(), "<a:wave:7>");
		assert!(valid_emoji_favorites(&[unicode.clone(), custom.clone()]));
		assert!(!valid_emoji_favorites(&[custom.clone(), custom]));
		assert!(
			!EmojiFavorite {
				id: Some(Id(0)),
				name: "wave".into(),
				animated: false
			}
			.valid()
		);
		assert!(
			!EmojiFavorite {
				id: None,
				name: "text".into(),
				animated: true
			}
			.valid()
		);
	}
}
