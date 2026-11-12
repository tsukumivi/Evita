//! Bounded on-demand server administration data; absent metadata stays unknown.
use crate::{CustomEmoji, Id, User, permissions};

pub const PAGE_SIZE: usize = 100;
pub const MAX_BYTES: usize = 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 256 * 1024;
pub const MAX_IMAGE_URI: usize = 24 + 4 * MAX_IMAGE_BYTES.div_ceil(3);
pub const MAX_STICKER_FILE_BYTES: usize = 512 * 1024;
pub const MAX_SOUNDBOARD_SOUNDS: usize = 1000;
pub const MAX_SOUNDBOARD_FILE_BYTES: usize = 512 * 1024;
pub const MAX_SOUNDBOARD_DURATION_MS: u16 = 5200;
pub const MEMBER_CHANNEL_FEATURE: &str = "ENABLED_MODERATION_EXPERIENCE_FOR_NON_COMMUNITY";
#[derive(Clone)]
pub struct Emoji {
	pub emoji: CustomEmoji,
	pub uploader: Option<User>,
}
#[derive(Clone, Default)]
pub struct Emojis {
	pub items: Vec<Emoji>,
	pub static_limit: Option<usize>,
	pub animated_limit: Option<usize>,
}
#[derive(Clone)]
pub struct Sticker {
	pub sticker: crate::Sticker,
	pub uploader: Option<User>,
}
#[derive(Clone, Default)]
pub struct Stickers {
	pub items: Vec<Sticker>,
	pub limit: Option<usize>,
}
#[derive(Clone)]
pub struct SoundboardSound {
	pub id: Id,
	pub guild_id: Option<Id>,
	pub name: String,
	/// Discord's normalized volume in the 0–100 range.
	pub volume: u8,
	pub emoji_id: Option<Id>,
	pub emoji_name: Option<String>,
	pub available: bool,
	pub user: Option<User>,
}
impl SoundboardSound {
	pub fn bytes(&self) -> usize {
		size_of::<Self>()
			+ self.name.capacity()
			+ self.emoji_name.as_ref().map_or(0, String::capacity)
			+ self.user.as_ref().map_or(0, User::heap_bytes)
	}
	pub fn valid(&self) -> bool {
		self.id.0 != 0
			&& (2..=32).contains(&self.name.chars().count())
			&& !self.name.chars().any(char::is_control)
			&& self.guild_id.is_none_or(|guild| guild.0 != 0)
			&& self.emoji_id.is_none_or(|emoji| emoji.0 != 0)
			&& self
				.emoji_name
				.as_ref()
				.is_none_or(|name| name.len() <= 64 && !name.chars().any(char::is_control))
			&& self
				.user
				.as_ref()
				.is_none_or(|user| user.id.0 != 0 && user.heap_bytes() <= 1024)
			&& self.volume <= 100
			&& self.bytes() <= 4096
	}
}
#[derive(Clone)]
pub struct Soundboard {
	pub guild: Id,
	pub items: Vec<SoundboardSound>,
}
impl Soundboard {
	pub fn bytes(&self) -> usize {
		size_of::<Self>()
			+ self.items.capacity() * size_of::<SoundboardSound>()
			+ self.items.iter().map(SoundboardSound::bytes).sum::<usize>()
	}
	pub fn valid(&self) -> bool {
		self.guild.0 != 0
			&& self.items.len() <= MAX_SOUNDBOARD_SOUNDS
			&& self.bytes() <= MAX_BYTES
			&& self.items.iter().enumerate().all(|(index, sound)| {
				sound.valid()
					&& sound.guild_id.is_none_or(|guild| guild == self.guild)
					&& !self.items[..index].iter().any(|other| other.id == sound.id)
			})
	}
}
#[derive(Clone)]
pub struct Member {
	pub user: User,
	pub nick: Option<String>,
	pub roles: Vec<Id>,
	pub joined_at: Option<i128>,
	pub join_source: Option<u8>,
	pub invite_code: Option<String>,
	pub flags: Option<u64>,
	pub unusual_dm_until: Option<i128>,
	pub timeout_until: Option<i128>,
}
#[derive(Clone)]
pub struct Role {
	pub role: permissions::Role,
	pub managed: bool,
}
#[derive(Clone, Default)]
pub struct Members {
	pub items: Vec<Member>,
	pub roles: Vec<Role>,
	pub total: u64,
	pub next: Option<Cursor>,
	pub features: Vec<String>,
	pub show_in_channel_list: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
	pub user: Id,
	pub joined_at: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
	pub search: String,
	pub sort: u8,
	pub recent: bool,
	pub after: Option<Cursor>,
}
impl Default for Query {
	fn default() -> Self {
		Self {
			search: String::new(),
			sort: 1,
			recent: false,
			after: None,
		}
	}
}
impl Query {
	pub fn valid(&self) -> bool {
		self.search.capacity() <= 400
			&& self.search.chars().count() <= 100
			&& !self.search.chars().any(char::is_control)
			&& (1..=4).contains(&self.sort)
			&& self
				.after
				.is_none_or(|cursor| cursor.user.0 != 0 && cursor.joined_at >= 0)
	}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceChange {
	Move(Option<Id>),
	Mute(bool),
}
#[derive(Clone)]
pub enum Action {
	Voice {
		user: Id,
		channel: Id,
		change: VoiceChange,
	},
	AuditLog(crate::server_audit_log::Query),
	Integrations(crate::server_integrations::Action),
	Invites(crate::server_invites::Action),
	Roles(crate::server_roles::Action),
	LoadEmojis,
	CreateEmoji {
		name: String,
		image: String,
	},
	RenameEmoji {
		id: Id,
		name: String,
	},
	DeleteEmoji {
		id: Id,
	},
	LoadStickers,
	LoadSoundboard {
		channel: Option<Id>,
	},
	CreateSoundboard {
		name: String,
		content_type: String,
		file: Vec<u8>,
		duration_ms: u16,
	},
	PlaySoundboard {
		channel: Id,
		sound: Id,
		source_guild: Id,
	},
	CreateSticker {
		name: String,
		description: String,
		tags: String,
		filename: String,
		content_type: String,
		file: Vec<u8>,
	},
	EditSticker {
		id: Id,
		name: String,
		description: String,
		tags: String,
	},
	DeleteSticker {
		id: Id,
	},
	LoadMembers(Query),
	SetRole {
		user: Id,
		role: Id,
		assigned: bool,
	},
	SetNickname {
		user: Id,
		nick: String,
	},
	Kick {
		user: Id,
	},
	Prune {
		days: u8,
		execute: bool,
	},
	ShowMembers {
		enabled: bool,
	},
}
impl Action {
	pub fn write(&self) -> bool {
		if let Self::Integrations(action) = self {
			return action.write();
		}
		if let Self::Invites(action) = self {
			return action.write();
		}
		if let Self::Roles(action) = self {
			return action.write();
		}
		!matches!(
			self,
			Self::AuditLog(_)
				| Self::LoadEmojis
				| Self::LoadStickers
				| Self::LoadSoundboard { .. }
				| Self::PlaySoundboard { .. }
				| Self::LoadMembers(_)
				| Self::Prune { execute: false, .. }
		)
	}
	pub fn sticker(&self) -> bool {
		matches!(
			self,
			Self::LoadStickers
				| Self::CreateSticker { .. }
				| Self::EditSticker { .. }
				| Self::DeleteSticker { .. }
		)
	}
	pub fn soundboard(&self) -> bool {
		matches!(
			self,
			Self::LoadSoundboard { .. }
				| Self::CreateSoundboard { .. }
				| Self::PlaySoundboard { .. }
		)
	}
	pub fn emoji(&self) -> bool {
		matches!(
			self,
			Self::LoadEmojis
				| Self::CreateEmoji { .. }
				| Self::RenameEmoji { .. }
				| Self::DeleteEmoji { .. }
		)
	}
	pub fn valid(&self) -> bool {
		match self {
			Self::Voice {
				user,
				channel,
				change,
			} => {
				user.0 != 0
					&& channel.0 != 0
					&& !matches!(change, VoiceChange::Move(Some(to)) if to.0 == 0 || to == channel)
			}
			Self::Roles(action) => action.valid(),
			Self::Invites(action) => action.valid(),
			Self::Integrations(action) => action.valid(),
			Self::AuditLog(query) => query.valid(),
			Self::CreateEmoji { name, image } => {
				name.capacity() <= 128
					&& valid_emoji_name(name)
					&& image.capacity() <= MAX_IMAGE_URI
					&& (image.starts_with("data:image/png;base64,")
						|| image.starts_with("data:image/gif;base64,"))
			}
			Self::RenameEmoji { id, name } => {
				id.0 != 0 && name.capacity() <= 128 && valid_emoji_name(name)
			}
			Self::DeleteEmoji { id } => id.0 != 0,
			Self::CreateSticker {
				name,
				description,
				tags,
				filename,
				content_type,
				file,
			} => {
				name.capacity() <= 120
					&& description.capacity() <= 400
					&& tags.capacity() <= 800
					&& valid_sticker_fields(name, description, tags)
					&& filename.capacity() <= 128
					&& (1..=128).contains(&filename.len())
					&& filename.bytes().all(|byte| {
						byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
					}) && content_type.capacity() <= 32
					&& matches!(
						(content_type.as_str(), filename.rsplit('.').next()),
						("image/png", Some("png"))
							| ("image/gif", Some("gif"))
							| ("application/json", Some("json"))
					) && !file.is_empty()
					&& file.len() <= MAX_STICKER_FILE_BYTES
					&& file.capacity() <= MAX_STICKER_FILE_BYTES
			}
			Self::EditSticker {
				id,
				name,
				description,
				tags,
			} => {
				id.0 != 0
					&& name.capacity() <= 120
					&& description.capacity() <= 400
					&& tags.capacity() <= 800
					&& valid_sticker_fields(name, description, tags)
			}
			Self::DeleteSticker { id } => id.0 != 0,
			Self::CreateSoundboard {
				name,
				content_type,
				file,
				duration_ms,
			} => {
				(2..=32).contains(&name.chars().count())
					&& !name.chars().any(char::is_control)
					&& (content_type == "audio/mpeg" || content_type == "audio/ogg")
					&& !file.is_empty()
					&& file.len() <= MAX_SOUNDBOARD_FILE_BYTES
					&& file.capacity() <= MAX_SOUNDBOARD_FILE_BYTES
					&& (1..=MAX_SOUNDBOARD_DURATION_MS).contains(duration_ms)
					&& if content_type == "audio/ogg" {
						file.starts_with(b"OggS")
					} else {
						file.starts_with(b"ID3") || file.first().is_some_and(|first| *first == 0xff)
					}
			}
			Self::PlaySoundboard {
				channel,
				sound,
				source_guild,
			} => channel.0 != 0 && sound.0 != 0 && source_guild.0 != 0,
			Self::LoadMembers(query) => query.valid(),
			Self::SetRole { user, role, .. } => user.0 != 0 && role.0 != 0,
			Self::SetNickname { user, nick } => {
				user.0 != 0
					&& nick.capacity() <= 128
					&& nick.chars().count() <= 32
					&& !nick.chars().any(char::is_control)
			}
			Self::Kick { user } => user.0 != 0,
			Self::Prune { days, .. } => (1..=30).contains(days),
			_ => true,
		}
	}
}
pub enum Result {
	VoiceUpdated(Id),
	WebhookUrl(crate::server_integrations::WebhookUrl),
	AuditLog(crate::server_audit_log::Page),
	Integrations(crate::server_integrations::Snapshot),
	Invites(crate::server_invites::Snapshot),
	Roles(crate::server_roles::Result),
	Emojis(Emojis),
	Stickers(Stickers),
	Soundboard(Soundboard),
	SoundboardPlayed,
	Members(Members),
	Member(Member),
	Kicked(Id),
	Pruned(Option<u64>),
	ChannelList(bool),
}
pub fn valid_emoji_name(name: &str) -> bool {
	(2..=32).contains(&name.len())
		&& name
			.bytes()
			.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
pub fn valid_sticker_fields(name: &str, description: &str, tags: &str) -> bool {
	(2..=30).contains(&name.chars().count())
		&& !name.chars().any(char::is_control)
		&& (description.is_empty() || (2..=100).contains(&description.chars().count()))
		&& !description.chars().any(char::is_control)
		&& (1..=200).contains(&tags.chars().count())
		&& !tags.chars().any(char::is_control)
}
impl Member {
	pub fn bytes(&self) -> usize {
		size_of::<Self>()
			+ self.user.heap_bytes()
			+ self.nick.as_ref().map_or(0, String::capacity)
			+ self.roles.capacity() * size_of::<Id>()
			+ self.invite_code.as_ref().map_or(0, String::capacity)
	}
	pub fn valid(&self) -> bool {
		self.user.id.0 != 0
			&& self.user.heap_bytes() <= 1024
			&& self.roles.len() <= permissions::MAX_MEMBER_ROLES
			&& self.roles.iter().all(|id| id.0 != 0)
			&& self.nick.as_ref().is_none_or(|nick| nick.len() <= 128)
			&& self
				.invite_code
				.as_ref()
				.is_none_or(|value| value.len() <= 128)
			&& [self.joined_at, self.unusual_dm_until, self.timeout_until]
				.into_iter()
				.all(|value| value.is_none_or(|value| (0..=i128::from(i64::MAX)).contains(&value)))
			&& self.bytes() <= 8192
	}
}
impl Result {
	pub fn bytes(&self) -> usize {
		size_of::<Self>()
			+ match self {
				Self::AuditLog(page) => page.bytes(),
				Self::WebhookUrl(url) => url.bytes(),
				Self::Integrations(snapshot) => snapshot.bytes(),
				Self::Roles(result) => result.bytes(),
				Self::Invites(snapshot) => snapshot.bytes(),
				Self::Emojis(page) => {
					page.items.capacity() * size_of::<Emoji>()
						+ page
							.items
							.iter()
							.map(|row| {
								row.emoji.heap_bytes()
									+ row.uploader.as_ref().map_or(0, User::heap_bytes)
							})
							.sum::<usize>()
				}
				Self::Stickers(page) => {
					page.items.capacity() * size_of::<Sticker>()
						+ page
							.items
							.iter()
							.map(|row| {
								row.sticker.heap_bytes()
									+ row.uploader.as_ref().map_or(0, User::heap_bytes)
							})
							.sum::<usize>()
				}
				Self::Soundboard(catalog) => catalog.bytes(),
				Self::Members(page) => page.bytes(),
				Self::Member(member) => member.bytes(),
				_ => 0,
			}
	}
	pub fn valid(&self) -> bool {
		self.bytes() <= MAX_BYTES
			&& match self {
				Self::AuditLog(page) => page.valid_response(),
				Self::Integrations(snapshot) => snapshot.valid(),
				Self::Roles(result) => result.valid(),
				Self::Invites(snapshot) => snapshot.valid(),
				Self::Emojis(page) => {
					page.items.len() <= crate::MAX_GUILD_EMOJIS
						&& page.items.capacity() * size_of::<CustomEmoji>()
							+ page
								.items
								.iter()
								.map(|row| row.emoji.heap_bytes())
								.sum::<usize>() <= crate::MAX_GUILD_EMOJI_BYTES
						&& page.items.iter().all(|row| {
							row.emoji.valid()
								&& row
									.uploader
									.as_ref()
									.is_none_or(|user| user.id.0 != 0 && user.heap_bytes() <= 1024)
						})
				}
				Self::Stickers(page) => {
					page.items.len() <= crate::MAX_GUILD_STICKERS
						&& page
							.limit
							.is_none_or(|limit| limit <= crate::MAX_GUILD_STICKERS)
						&& page
							.items
							.iter()
							.map(|row| row.sticker.heap_bytes() + size_of::<crate::Sticker>())
							.sum::<usize>() <= crate::MAX_STICKER_BYTES
						&& page.items.iter().enumerate().all(|(index, row)| {
							row.sticker.valid()
								&& row.sticker.guild_id.is_some()
								&& row.sticker.pack_id.is_none()
								&& matches!(row.sticker.format_type, 1..=4)
								&& !page.items[..index]
									.iter()
									.any(|other| other.sticker.id == row.sticker.id)
								&& row
									.uploader
									.as_ref()
									.is_none_or(|user| user.id.0 != 0 && user.heap_bytes() <= 1024)
						})
				}
				Self::Soundboard(catalog) => catalog.valid(),
				Self::Members(page) => page.valid(),
				Self::Member(member) => member.valid(),
				Self::Kicked(id) | Self::VoiceUpdated(id) => id.0 != 0,
				_ => true,
			}
	}
}

impl Members {
	pub fn bytes(&self) -> usize {
		self.items.iter().map(Member::bytes).sum::<usize>()
			+ self.items.capacity().saturating_sub(self.items.len()) * size_of::<Member>()
			+ self.roles.capacity() * size_of::<Role>()
			+ self
				.roles
				.iter()
				.map(|role| role.role.name.capacity())
				.sum::<usize>()
			+ self.features.capacity() * size_of::<String>()
			+ self.features.iter().map(String::capacity).sum::<usize>()
	}
	pub fn valid(&self) -> bool {
		self.bytes() <= MAX_BYTES
			&& self.items.len() <= PAGE_SIZE
			&& self.items.iter().all(Member::valid)
			&& self.roles.len() <= permissions::MAX_ROLES
			&& self
				.roles
				.iter()
				.all(|role| role.role.id.0 != 0 && role.role.name.len() <= 400)
			&& self.features.len() <= 256
			&& self.features.iter().all(|value| value.len() <= 128)
	}
}

#[cfg(test)]
mod soundboard_action_tests {
	use super::*;

	fn action(duration_ms: u16) -> Action {
		Action::CreateSoundboard {
			name: "Wave".into(),
			content_type: "audio/ogg".into(),
			file: b"OggSdata".to_vec(),
			duration_ms,
		}
	}

	#[test]
	fn soundboard_upload_requires_bounded_supported_audio_and_duration() {
		assert!(action(5200).valid());
		assert!(!action(5201).valid());
		assert!(!action(0).valid());
		let mut invalid = action(1000);
		if let Action::CreateSoundboard { content_type, .. } = &mut invalid {
			*content_type = "audio/wav".into();
		}
		assert!(!invalid.valid());
	}
}
