//! Guild-scoped soundboard management and upload review.
use client_core::{Command, State};
use egui::RichText;
use model::{Id, server_admin::Action};

pub(super) type PreparedSound = (String, String, Vec<u8>, u16);

#[derive(Default)]
pub(super) struct SoundboardUi {
	pub request: u64,
	choosing: bool,
	request_started: bool,
	prepared: Option<PreparedSound>,
	submitted: bool,
	error: Option<&'static str>,
}

impl SoundboardUi {
	pub fn has_changes(&self) -> bool {
		self.choosing || self.prepared.is_some() || self.submitted
	}

	pub fn load(&mut self, state: &mut State, guild: Id) -> Option<Command> {
		if state.server_admin.pending
			|| (state.server_admin.guild == Some(guild)
				&& (state.server_admin.error.is_some() || state.server_admin.soundboard.is_some()))
		{
			return None;
		}
		state.request_server_admin(guild, Action::LoadSoundboard { channel: None })
	}

	pub fn choose(&mut self) {
		if !self.choosing && self.prepared.is_none() {
			self.choosing = true;
			self.error = None;
		}
	}

	pub fn take_request(&mut self) -> bool {
		if !self.choosing || self.request_started {
			return false;
		}
		self.request_started = true;
		true
	}

	pub fn accept(&mut self, result: Result<Option<PreparedSound>, &'static str>) {
		if !self.choosing || !self.request_started {
			return;
		}
		self.choosing = false;
		self.request_started = false;
		match result {
			Ok(Some(sound)) => self.prepared = Some(sound),
			Ok(None) => {}
			Err(error) => self.error = Some(error),
		}
	}

	pub fn show(
		&mut self,
		ui: &mut egui::Ui,
		state: &mut State,
		guild: Id,
		commands: &mut Vec<Command>,
	) {
		if self.submitted && !state.server_admin.pending {
			self.submitted = false;
			if state.server_admin.error.is_none() {
				self.prepared = None;
			}
		}
		if !state.can_create_guild_soundboard(guild) {
			self.prepared = None;
			self.choosing = false;
			self.request_started = false;
		}

		crate::design::page_header(
			ui,
			"server-settings-page-soundboard",
			Some("server-soundboard-subtitle"),
			|ui| {
				if state.can_create_guild_soundboard(guild)
					&& ui
						.add_enabled_ui(
							!self.choosing
								&& self.prepared.is_none()
								&& !state.server_admin.pending,
							|ui| {
								crate::design::button(
									ui,
									"server-soundboard-upload",
									crate::design::ButtonKind::Primary,
								)
							},
						)
						.inner
						.clicked()
				{
					self.choose();
				}
			},
		);

		if let Some(error) = state.server_admin.error.or(self.error) {
			crate::design::notice(ui, crate::design::Level::Error, error);
			if ui
				.add_enabled(
					!state.server_admin.pending,
					egui::Button::new(crate::i18n::translate_if_key("server-soundboard-reload")),
				)
				.clicked() && let Some(command) =
				state.request_server_admin(guild, Action::LoadSoundboard { channel: None })
			{
				commands.push(command);
				self.error = None;
			}
		}
		if state.server_admin.pending {
			ui.horizontal(|ui| {
				ui.spinner();
				ui.weak(if state.server_admin.saving {
					crate::i18n::translate_if_key("server-soundboard-uploading")
				} else {
					crate::i18n::translate_if_key("server-soundboard-loading")
				});
			});
		}
		if self.choosing {
			ui.weak(crate::i18n::translate_if_key(
				"server-soundboard-file-guidance",
			));
		}
		if let Some((name, content_type, file, duration_ms)) = &mut self.prepared {
			let mut cancel = false;
			crate::design::group(ui, "server-soundboard-review", |ui| {
				ui.label(crate::i18n::translate_if_key("server-soundboard-name"));
				ui.add(egui::TextEdit::singleline(name).char_limit(32));
				ui.label(
					RichText::new(format!(
						"{} · {:.1}s · {} KB",
						content_type,
						f64::from(*duration_ms) / 1000.0,
						file.len().div_ceil(1024),
					))
					.size(12.0)
					.color(crate::design::palette(ui).muted),
				);
				ui.horizontal(|ui| {
					let valid_name = (2..=32).contains(&name.trim().chars().count())
						&& !name.chars().any(char::is_control);
					if ui
						.add_enabled_ui(valid_name && !state.server_admin.pending, |ui| {
							crate::design::button(
								ui,
								"server-soundboard-add",
								crate::design::ButtonKind::Primary,
							)
						})
						.inner
						.clicked() && let Some(command) = state.request_server_admin(
						guild,
						Action::CreateSoundboard {
							name: name.trim().to_owned(),
							content_type: content_type.clone(),
							file: file.clone(),
							duration_ms: *duration_ms,
						},
					) {
						self.submitted = true;
						commands.push(command);
					}
					if ui
						.add_enabled_ui(!state.server_admin.pending, |ui| {
							crate::design::button(
								ui,
								"server-soundboard-cancel",
								crate::design::ButtonKind::Neutral,
							)
						})
						.inner
						.clicked()
					{
						cancel = true;
					}
				});
			});
			if cancel {
				self.prepared = None;
			}
		}

		ui.add_space(20.0);
		let Some(catalog) = state
			.server_admin
			.soundboard
			.as_ref()
			.filter(|catalog| catalog.guild == guild)
		else {
			if !state.server_admin.pending
				&& state.server_admin.error.is_none()
				&& let Some(command) = self.load(state, guild)
			{
				commands.push(command);
			}
			return;
		};
		crate::design::section(
			ui,
			"server-soundboard-server-sounds",
			Some(&crate::i18n::translate_count(
				"server-soundboard-sound-count",
				catalog.items.len(),
				&[],
			)),
		);
		if catalog.items.is_empty() {
			crate::design::empty_state(
				ui,
				crate::icons::Icon::Soundboard,
				"server-soundboard-empty",
				"",
			);
			return;
		}
		for sound in &catalog.items {
			ui.horizontal(|ui| {
				ui.label(sound.emoji_name.as_deref().unwrap_or("🔊"));
				ui.label(&sound.name);
				ui.weak(format!("{}%", sound.volume));
				if !sound.available {
					ui.weak(crate::i18n::translate_if_key(
						"server-soundboard-unavailable",
					));
				}
			});
		}
	}
}
