//! Device-local pinned and favorite channels as sidebar rosters. Menus record an `Intent`;
//! `MessagingUi::apply_shortcut` is the only writer of `channel_preferences` in this crate.
use crate::MessagingUi;
use client_core::State;
use model::{Channel, ChannelPreferences, Id, PreferenceEdit, Shortcut};
use std::collections::{BTreeMap, BTreeSet};

/// Which sidebar list is being built. Owns every home-vs-guild policy for shortcuts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
	Home,
	Guild(Id),
}

impl Scope {
	pub fn of(guild: Option<Id>) -> Self {
		guild.map_or(Self::Home, Self::Guild)
	}

	pub fn guild(self) -> Option<Id> {
		match self {
			Self::Home => None,
			Self::Guild(guild) => Some(guild),
		}
	}

	/// Channels this list may show at all, before permission filtering.
	pub fn admits(self, channel: &Channel, state: &State) -> bool {
		channel.guild == self.guild()
			&& channel.kind != 4
			&& (self.guild().is_some()
				|| (!state.spam_direct(channel.id) && !state.message_request(channel.id)))
	}

	/// Rosters in display order. Home keeps pinned conversations and favorites; guilds keep their
	/// favorite channels in the same shelf.
	pub fn rosters(self) -> &'static [(Shortcut, Heading)] {
		match self {
			Self::Home => &[
				(Shortcut::Pinned, Heading::Pinned),
				(Shortcut::Favorite, Heading::Favorites),
			],
			Self::Guild(_) => &[(Shortcut::Favorite, Heading::Favorites)],
		}
	}

	/// The heading over the non-roster remainder, if that list has one.
	pub fn remainder_heading(self) -> Option<Heading> {
		match self {
			Self::Home => Some(Heading::DirectMessages),
			Self::Guild(_) => None,
		}
	}
}

/// Typed heading rows. `key` identifies the localized user-facing string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Heading {
	MessageRequests,
	Pinned,
	DirectMessages,
	Favorites,
}

impl Heading {
	pub fn key(self) -> &'static str {
		match self {
			Self::MessageRequests => "channel-list-heading-message-requests",
			Self::Pinned => "channel-list-heading-pinned",
			Self::DirectMessages => "channel-list-heading-direct-messages",
			Self::Favorites => "channel-list-heading-favorites",
		}
	}

	pub fn shelf(self) -> bool {
		matches!(self, Self::Pinned | Self::Favorites)
	}
}

/// One heading and its display-ordered channels.
pub(crate) struct Section<'a> {
	pub heading: Heading,
	pub kind: Shortcut,
	pub channels: Vec<&'a Channel>,
}

/// Roster rows for one scope, derived from preferences whenever the row cache rebuilds.
/// A channel in both lists appears once, under the first roster in `Scope::rosters`.
#[derive(Default)]
pub(crate) struct Roster<'a> {
	sections: Vec<Section<'a>>,
	lifted: BTreeSet<Id>,
}

impl<'a> Roster<'a> {
	pub fn build(
		state: &'a State,
		prefs: &ChannelPreferences,
		scope: Scope,
		show_hidden: bool,
	) -> Self {
		let rosters = scope.rosters();
		let mut rank = BTreeMap::new();
		for (index, (kind, _)) in rosters.iter().enumerate() {
			for (position, id) in prefs.ids(*kind).iter().enumerate() {
				rank.entry(*id).or_insert((index, position));
			}
		}
		let mut buckets: Vec<Vec<(usize, &Channel)>> = vec![Vec::new(); rosters.len()];
		for channel in &state.channels {
			if let Some(&(index, position)) = rank.get(&channel.id)
				&& scope.admits(channel, state)
				&& (show_hidden || state.can_view(channel.id))
			{
				buckets[index].push((position, channel));
			}
		}
		let mut lifted = BTreeSet::new();
		let sections = rosters
			.iter()
			.zip(buckets)
			.filter(|(_, bucket)| !bucket.is_empty())
			.map(|(&(kind, heading), mut bucket)| {
				bucket.sort_unstable_by_key(|(position, channel)| (*position, channel.id));
				lifted.extend(bucket.iter().map(|(_, channel)| channel.id));
				Section {
					heading,
					kind,
					channels: bucket.into_iter().map(|(_, channel)| channel).collect(),
				}
			})
			.collect();
		Self { sections, lifted }
	}

	pub fn sections(&self) -> &[Section<'a>] {
		&self.sections
	}

	/// Whether the regular list skips this channel. True for home pins and guild favorites.
	pub fn lifted(&self, channel: Id) -> bool {
		self.lifted.contains(&channel)
	}
}

/// A deferred local write recorded by a menu, applied once rows released their borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Intent {
	pub channel: Id,
	pub kind: Shortcut,
	pub on: bool,
}

/// Read-only handle menus use to label and enable shortcut rows.
#[derive(Clone, Copy)]
pub(crate) struct ShortcutView<'a> {
	prefs: &'a ChannelPreferences,
	available: bool,
}

impl<'a> ShortcutView<'a> {
	pub fn new(prefs: &'a ChannelPreferences, available: bool) -> Self {
		Self { prefs, available }
	}

	pub fn contains(self, kind: Shortcut, channel: Id) -> bool {
		self.prefs.contains(kind, channel)
	}

	pub fn available(self) -> bool {
		self.available
	}

	/// The intent that flips `kind` for `channel` from its current state.
	pub fn toggle(self, kind: Shortcut, channel: Id) -> Intent {
		Intent {
			channel,
			kind,
			on: !self.contains(kind, channel),
		}
	}
}

impl MessagingUi {
	/// Global Favorites page. The device-local list can mix guild channels with DMs, so it lives
	/// outside the per-server channel roster.
	pub(super) fn favorites_page(
		&mut self,
		ui: &mut egui::Ui,
		state: &mut State,
		commands: &mut Vec<client_core::Command>,
	) {
		let colors = crate::design::palette(ui);
		let language = self.language;
		let favorites = self.channel_preferences.favorites.clone();
		let mut selected = None;
		let mut remove = None;
		egui::Frame::new()
			.inner_margin(egui::Margin::symmetric(28, 24))
			.show(ui, |ui| {
				ui.label(
					crate::design::semibold(ui, language.text("favorites-title"), 24.0)
						.color(colors.text_strong),
				);
				ui.add_space(4.0);
				ui.label(
					egui::RichText::new(language.text("favorites-description")).color(colors.muted),
				);
				ui.add_space(20.0);
				let available: Vec<_> = favorites
					.iter()
					.filter_map(|id| state.channel(*id))
					.filter(|channel| {
						ChannelPreferences::can_favorite(channel)
							&& (state.demo || state.can_view(channel.id))
					})
					.collect();
				if available.is_empty() {
					crate::design::empty_state(
						ui,
						crate::icons::Icon::Star,
						&language.text("favorites-empty-title"),
						&language.text("favorites-empty-description"),
					);
					return;
				}
				let mut previous_group = None;
				for channel in available {
					let group = channel
						.guild
						.and_then(|guild| state.guild(guild))
						.map_or_else(
							|| language.text("channel-list-heading-direct-messages"),
							|guild| guild.name.clone(),
						);
					if previous_group.as_deref() != Some(group.as_str()) {
						ui.label(crate::design::eyebrow(ui, &group, colors.muted));
						previous_group = Some(group);
						ui.add_space(5.0);
					}
					let (rect, response) = ui.allocate_exact_size(
						egui::vec2(ui.available_width(), 48.0),
						egui::Sense::click(),
					);
					if response.hovered() || response.has_focus() {
						ui.painter().rect_filled(rect, 6, colors.hover);
					}
					let icon = match channel.kind {
						2 => crate::icons::Icon::Speaker,
						15 | 16 => crate::icons::Icon::Forum,
						_ => crate::icons::Icon::Hash,
					};
					crate::icons::paint(
						ui.painter(),
						icon,
						egui::Rect::from_center_size(
							egui::pos2(rect.left() + 18.0, rect.center().y),
							egui::vec2(18.0, 18.0),
						),
						colors.muted,
					);
					let title = if channel.guild.is_none() && channel.name.is_empty() {
						channel
							.recipients
							.iter()
							.map(|user| state.user_display_name(user))
							.collect::<Vec<_>>()
							.join(", ")
					} else {
						channel.name.clone()
					};
					ui.painter().text(
						egui::pos2(rect.left() + 36.0, rect.center().y),
						egui::Align2::LEFT_CENTER,
						&title,
						egui::FontId::new(14.0, crate::design::body_family(ui.ctx())),
						colors.text,
					);
					response.widget_info(|| {
						egui::WidgetInfo::labeled(egui::Role::Button, true, &title)
					});
					response.context_menu(|ui| {
						if ui
							.button(language.text("channel-menu-context-remove-from-favorites"))
							.clicked()
						{
							remove = Some(channel.id);
							ui.close();
						}
					});
					if response.clicked() {
						selected = Some(channel.id);
					}
					ui.add_space(2.0);
				}
			});
		if let Some(channel) = remove {
			self.apply_shortcut(
				state,
				Intent {
					channel,
					kind: Shortcut::Favorite,
					on: false,
				},
			);
		}
		if let Some(channel) = selected {
			self.home_page = super::HomePage::Friends;
			self.guild = state.channel(channel).and_then(|channel| channel.guild);
			if let Some(command) = state.select(channel) {
				commands.push(command);
			}
		}
	}

	/// Whether preferences are loaded (or the session is a demo) so a write would stick.
	pub(crate) fn shortcuts_available(&self, state: &State) -> bool {
		state.demo || self.channel_preferences_loaded
	}

	pub(crate) fn apply_shortcut(&mut self, state: &State, intent: Intent) {
		if !self.shortcuts_available(state) {
			return;
		}
		if intent.on {
			let Some(channel) = state.channel(intent.channel) else {
				return;
			};
			let allowed = match intent.kind {
				Shortcut::Pinned => ChannelPreferences::can_pin(channel),
				Shortcut::Favorite => ChannelPreferences::can_favorite(channel),
			};
			if !allowed {
				return;
			}
		}
		match self
			.channel_preferences
			.set(intent.kind, intent.channel, intent.on)
		{
			PreferenceEdit::Changed => {
				self.channel_preferences_changed = true;
				self.channel_cache.invalidate();
			}
			PreferenceEdit::CapacityReached => self.channel_menu.report_capacity(state.generation),
			PreferenceEdit::Unchanged => {}
		}
	}
}
