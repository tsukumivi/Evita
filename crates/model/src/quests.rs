//! Bounded data used to render the signed-in account's quest cards.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quest {
	pub id: String,
	pub title: String,
	pub description: String,
	pub progress: String,
	/// Rounded completion percentage when Discord returns a current and target count.
	pub progress_percent: Option<u8>,
	pub image: Option<String>,
}

impl Quest {
	pub fn valid(&self) -> bool {
		!self.id.is_empty()
			&& self.id.len() <= 64
			&& !self.title.is_empty()
			&& self.title.len() <= 256
			&& self.description.len() <= 1024
			&& self.progress.len() <= 128
			&& self.progress_percent.is_none_or(|percent| percent <= 100)
			&& self.image.as_ref().is_none_or(|url| {
				url.len() <= 2048
					&& (url.starts_with("https://cdn.discordapp.com/")
						|| url.starts_with("https://media.discordapp.net/"))
			})
	}

	pub fn bytes(&self) -> usize {
		std::mem::size_of::<Self>()
			+ self.id.capacity()
			+ self.title.capacity()
			+ self.description.capacity()
			+ self.progress.capacity()
			+ self.image.as_ref().map_or(0, String::capacity)
	}
}
