//! One bounded, session-only draft for the account's global profile.
use crate::{avatars::Avatars, design, icons};
use client_core::{Command, State};
use model::{Id, ProfileEdit, UserProfile};

#[derive(Clone, PartialEq, Eq)]
struct Draft {
	name: String,
	bio: String,
	pronouns: String,
	color: Option<u32>,
	/// `Some(Some(uri))` uploads a new picture, `Some(None)` removes the current one.
	avatar: Option<Option<String>>,
}

#[cfg(test)]
mod crop_tests {
	use super::*;

	#[test]
	fn avatar_crop_controls_select_different_square_regions() {
		let ctx = egui::Context::default();
		let source = egui::ColorImage::new(
			[4, 2],
			(0..8)
				.map(|index| {
					if index % 4 < 2 {
						egui::Color32::RED
					} else {
						egui::Color32::BLUE
					}
				})
				.collect(),
		);
		let mut crop = CropDraft {
			texture: ctx.load_texture("crop-test", source.clone(), egui::TextureOptions::LINEAR),
			source,
			zoom: 1.0,
			pan: egui::Vec2::new(-1.0, 0.0),
			error: None,
		};
		let (left_uri, left) = encode_avatar_crop(&crop).unwrap();
		crop.pan.x = 1.0;
		let (right_uri, right) = encode_avatar_crop(&crop).unwrap();
		assert!(model::valid_avatar_uri(&left_uri));
		assert!(model::valid_avatar_uri(&right_uri));
		assert_eq!(left.size, [256, 256]);
		assert!(left.pixels[128 * 256 + 128].r() > left.pixels[128 * 256 + 128].b());
		assert!(right.pixels[128 * 256 + 128].b() > right.pixels[128 * 256 + 128].r());
	}
}
impl Draft {
	fn from_profile(profile: &UserProfile) -> Self {
		Self {
			name: profile.global_name.clone().unwrap_or_default(),
			bio: profile.bio.clone(),
			pronouns: profile.pronouns.clone(),
			color: profile.accent_color,
			avatar: None,
		}
	}
	fn changes(&self, profile: &UserProfile) -> ProfileEdit {
		let name = (!self.name.is_empty()).then(|| self.name.clone());
		ProfileEdit {
			global_name: (name != profile.global_name).then_some(name),
			bio: (self.bio != profile.bio).then(|| self.bio.clone()),
			pronouns: (self.pronouns != profile.pronouns).then(|| self.pronouns.clone()),
			accent_color: (self.color != profile.accent_color).then_some(self.color),
			avatar: match &self.avatar {
				// Removing a picture the account does not have is not a change.
				Some(None) if profile.user.avatar.is_none() => None,
				other => other.clone(),
			},
		}
	}
	fn rebase(&mut self, before: &Self, fresh: &Self) {
		if self.name == before.name {
			self.name = fresh.name.clone();
		}
		if self.bio == before.bio {
			self.bio = fresh.bio.clone();
		}
		if self.pronouns == before.pronouns {
			self.pronouns = fresh.pronouns.clone();
		}
		if self.color == before.color {
			self.color = fresh.color;
		}
	}
}

/// What the preview card paints inside the avatar circle.
enum Picture<'a> {
	/// The account's current picture, fetched like any other avatar.
	Remote,
	/// A locally chosen image that has not been saved yet.
	Pending(&'a egui::TextureHandle),
	/// A locally chosen image that was just saved; shown until the CDN copy is fetched.
	Saved(&'a egui::TextureHandle),
	/// Removal is pending, so the initial placeholder is shown.
	Removed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AvatarAction {
	Pick,
	Remove,
	Undo,
}

struct CropDraft {
	source: egui::ColorImage,
	texture: egui::TextureHandle,
	zoom: f32,
	pan: egui::Vec2,
	error: Option<&'static str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CropDecision {
	Cancel,
	Apply,
}

#[derive(Default)]
pub(super) struct Editor {
	generation: Option<u64>,
	draft: Option<Draft>,
	baseline: Option<Draft>,
	submitted: Option<u64>,
	saved: bool,
	preview_link: Option<String>,
	/// Set when the picture button is pressed; the desktop shell opens the native picker.
	pub avatar_request: Option<(u64, Id, u64)>,
	revision: u64,
	choosing: bool,
	pending_picture: Option<egui::TextureHandle>,
	/// Local copy of the last saved picture, keyed by the avatar hash the service returned.
	saved_picture: Option<(Option<String>, egui::TextureHandle)>,
	crop: Option<CropDraft>,
}
impl Editor {
	/// Result of the native picker. Errors are returned so the caller can toast them.
	pub fn accept_avatar(
		&mut self,
		ctx: &egui::Context,
		request: (u64, Id, u64),
		result: Result<Option<(String, egui::ColorImage)>, &'static str>,
	) -> Result<(), &'static str> {
		if !self.choosing
			|| request
				!= (
					self.generation.unwrap_or_default(),
					request.1,
					self.revision,
				) {
			return Ok(());
		}
		self.choosing = false;
		match result {
			Ok(Some((uri, image))) => {
				if image.size[0] == 0
					|| image.size[1] == 0
					|| image.size[0] > 512
					|| image.size[1] > 512
					|| (!uri.is_empty() && !model::valid_avatar_uri(&uri))
				{
					return Err("That picture is too large; choose a simpler image");
				}
				if uri.is_empty() {
					self.crop = Some(CropDraft {
						texture: ctx.load_texture(
							"profile-avatar-crop-source",
							image.clone(),
							egui::TextureOptions::LINEAR,
						),
						source: image,
						zoom: 1.0,
						pan: egui::Vec2::ZERO,
						error: None,
					});
					return Ok(());
				}
				let Some(draft) = self.draft.as_mut() else {
					return Ok(());
				};
				draft.avatar = Some(Some(uri));
				self.pending_picture = Some(ctx.load_texture(
					"profile-avatar-preview",
					image,
					egui::TextureOptions::LINEAR,
				));
				self.saved = false;
				Ok(())
			}
			Ok(None) => Ok(()),
			Err(error) => Err(error),
		}
	}

	pub fn show(
		&mut self,
		ui: &mut egui::Ui,
		state: &mut State,
		avatars: &mut Avatars,
		commands: &mut Vec<Command>,
	) {
		if self.generation != Some(state.generation) {
			*self = Self {
				generation: Some(state.generation),
				..Self::default()
			};
		}
		if state.own_profile.data.is_none()
			&& !state.own_profile.loading
			&& state.own_profile.error.is_none()
			&& let Some(command) = state.load_own_profile()
		{
			commands.push(command);
		}
		if self.submitted == Some(state.own_profile.request) && !state.own_profile.saving {
			self.submitted = None;
			if state.own_profile.error.is_none()
				&& let Some(profile) = &state.own_profile.data
			{
				let uploaded = self
					.draft
					.as_ref()
					.is_some_and(|draft| matches!(draft.avatar, Some(Some(_))));
				self.saved_picture = match (uploaded, self.pending_picture.take()) {
					(true, Some(texture)) => Some((profile.user.avatar.clone(), texture)),
					_ => None,
				};
				self.draft = Some(Draft::from_profile(profile));
				self.saved = true;
			}
		}
		let colors = design::palette(ui);
		if state.own_profile.loading {
			ui.horizontal(|ui| {
				ui.add(egui::Spinner::new().size(14.0));
				ui.label(
					egui::RichText::new(crate::i18n::translate(
						"profile-edit-show-loading-your-profile",
					))
					.size(13.0)
					.color(colors.muted),
				);
			});
		}
		if let Some(error) = state.own_profile.error {
			design::notice(ui, design::Level::Error, error);
			if !state.own_profile.loading
				&& !state.own_profile.saving
				&& design::text_action(
					ui,
					&crate::i18n::translate("profile-edit-show-reload-profile"),
				)
				.clicked() && let Some(command) = state.load_own_profile()
			{
				commands.push(command);
			}
		}
		let Some(profile) = state.own_profile.data.as_ref() else {
			return;
		};
		let fresh = Draft::from_profile(profile);
		if let (Some(draft), Some(before)) = (&mut self.draft, &self.baseline) {
			draft.rebase(before, &fresh);
		}
		self.baseline = Some(fresh);
		let crop_choice = self
			.crop
			.as_mut()
			.and_then(|crop| crop_dialog(ui.ctx(), crop));
		let mut cropped_avatar = None;
		match crop_choice {
			Some(CropDecision::Cancel) => self.crop = None,
			Some(CropDecision::Apply) => {
				if let Some(crop) = self.crop.as_ref() {
					match encode_avatar_crop(crop) {
						Ok(result) => {
							cropped_avatar = Some(result);
							self.crop = None;
						}
						Err(error) => {
							if let Some(crop) = self.crop.as_mut() {
								crop.error = Some(error);
							}
						}
					}
				}
			}
			None => {}
		}
		let draft = self
			.draft
			.get_or_insert_with(|| Draft::from_profile(profile));
		if let Some((uri, preview)) = cropped_avatar {
			draft.avatar = Some(Some(uri));
			self.pending_picture = Some(ui.ctx().load_texture(
				"profile-avatar-preview",
				preview,
				egui::TextureOptions::LINEAR,
			));
			self.saved = false;
		}
		let before = draft.clone();
		let editable = !state.own_profile.loading && !state.own_profile.saving;
		let picture = match &draft.avatar {
			Some(Some(_)) => self
				.pending_picture
				.as_ref()
				.map_or(Picture::Remote, Picture::Pending),
			Some(None) => Picture::Removed,
			None => match &self.saved_picture {
				Some((hash, texture)) if hash.is_some() && *hash == profile.user.avatar => {
					Picture::Saved(texture)
				}
				_ => Picture::Remote,
			},
		};
		let mut action = None;
		let width = ui.available_width();
		if width >= 900.0 {
			ui.horizontal_top(|ui| {
				ui.spacing_mut().item_spacing.x = 16.0;
				ui.allocate_ui_with_layout(
					egui::vec2(300.0, 0.0),
					egui::Layout::top_down(egui::Align::Min),
					|ui| {
						ui.add_enabled_ui(editable, |ui| {
							action = form(ui, draft, profile, self.choosing);
						});
					},
				);
				ui.allocate_ui_with_layout(
					egui::vec2(300.0, 0.0),
					egui::Layout::top_down(egui::Align::Min),
					|ui| {
						if let Some(picked) = preview(
							ui,
							draft,
							profile,
							avatars,
							(state.demo, &state.guilds),
							&mut self.preview_link,
							picture,
							editable && !self.choosing,
						) {
							action = Some(picked);
						}
					},
				);
				ui.allocate_ui_with_layout(
					egui::vec2((width - 632.0).max(240.0), 0.0),
					egui::Layout::top_down(egui::Align::Min),
					|ui| {
						crate::profiles::board_widgets_readonly(ui, profile, avatars, state.demo);
					},
				);
			});
		} else if width >= 620.0 {
			ui.horizontal_top(|ui| {
				ui.spacing_mut().item_spacing.x = 24.0;
				ui.allocate_ui_with_layout(
					egui::vec2(width - 324.0, 0.0),
					egui::Layout::top_down(egui::Align::Min),
					|ui| {
						ui.add_enabled_ui(editable, |ui| {
							action = form(ui, draft, profile, self.choosing);
						});
					},
				);
				ui.vertical(|ui| {
					ui.set_width(300.0);
					if let Some(picked) = preview(
						ui,
						draft,
						profile,
						avatars,
						(state.demo, &state.guilds),
						&mut self.preview_link,
						picture,
						editable && !self.choosing,
					) {
						action = Some(picked);
					}
				});
			});
		} else {
			ui.add_enabled_ui(editable, |ui| {
				action = form(ui, draft, profile, self.choosing);
			});
		}
		match action {
			Some(AvatarAction::Pick) if editable && !self.choosing => {
				self.revision = self.revision.wrapping_add(1);
				self.choosing = true;
				self.avatar_request = Some((state.generation, profile.user.id, self.revision));
			}
			Some(AvatarAction::Remove) => {
				draft.avatar = Some(None);
				self.pending_picture = None;
			}
			Some(AvatarAction::Undo) => {
				draft.avatar = None;
				self.pending_picture = None;
			}
			_ => {}
		}
		if *draft != before {
			self.saved = false;
		}
		let changes = draft.changes(profile);
		let changed = changes != ProfileEdit::default();
		if !changes.valid() {
			design::notice(
				ui,
				design::Level::Error,
				&crate::i18n::translate(
					"profile-edit-show-check-character-limits-and-remove-control-characters-a-display-name",
				),
			);
		}
		ui.add_space(16.0);
		egui::Frame::new()
			.fill(colors.base)
			.corner_radius(8)
			.inner_margin(egui::Margin::symmetric(12, 8))
			.show(ui, |ui| {
				ui.set_width(ui.available_width());
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 12.0;
					if state.own_profile.saving {
						ui.add(egui::Spinner::new().size(14.0));
						ui.label(
							egui::RichText::new(crate::i18n::translate(
								"profile-edit-show-saving-profile",
							))
							.size(13.0)
							.color(colors.muted),
						);
					} else if self.saved {
						ui.label(
							egui::RichText::new(crate::i18n::translate_if_key(if state.demo {
								"profile-edit-show-saved-in-preview"
							} else {
								"profile-edit-show-profile-saved"
							}))
							.size(13.0)
							.color(colors.positive),
						);
					} else if changed {
						ui.label(
							egui::RichText::new(crate::i18n::translate(
								"profile-edit-show-you-have-unsaved-changes",
							))
							.size(13.0)
							.color(colors.text),
						);
					}
					ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
						ui.add_enabled_ui(
							changed && changes.valid() && state.can_save_own_profile(),
							|ui| {
								if design::button(
									ui,
									&crate::i18n::translate("profile-edit-show-save-changes"),
									design::ButtonKind::Primary,
								)
								.clicked() && let Some(command) = state.save_own_profile(changes)
								{
									self.submitted = Some(state.own_profile.request);
									self.saved = false;
									commands.push(command);
								}
							},
						);
						ui.add_enabled_ui(changed && editable, |ui| {
							if design::button(
								ui,
								&crate::i18n::translate("profile-edit-show-cancel"),
								design::ButtonKind::Neutral,
							)
							.clicked()
							{
								self.draft =
									state.own_profile.data.as_ref().map(Draft::from_profile);
								self.pending_picture = None;
								self.saved = false;
							}
						});
					});
				});
			});
		if width < 620.0
			&& let (Some(draft), Some(profile)) = (&self.draft, &state.own_profile.data)
		{
			ui.add_space(16.0);
			let picture = match &draft.avatar {
				Some(Some(_)) => self
					.pending_picture
					.as_ref()
					.map_or(Picture::Remote, Picture::Pending),
				Some(None) => Picture::Removed,
				None => Picture::Remote,
			};
			preview(
				ui,
				draft,
				profile,
				avatars,
				(state.demo, &state.guilds),
				&mut self.preview_link,
				picture,
				false,
			);
		}
		crate::markdown::confirm_external_link(ui.ctx(), &mut self.preview_link, true);
		if !state.demo && !state.gateway_connected {
			design::hint(
				ui,
				&crate::i18n::translate("profile-edit-show-reconnect-to-save-your-profile"),
			);
		}
	}
}

fn crop_dialog(ctx: &egui::Context, crop: &mut CropDraft) -> Option<CropDecision> {
	let colors = design::palette_for(ctx);
	let mut decision = None;
	egui::Modal::new(egui::Id::new("profile-avatar-crop"))
		.frame(
			egui::Frame::new()
				.fill(colors.chat)
				.stroke(egui::Stroke::new(1.0, colors.border))
				.corner_radius(12)
				.inner_margin(20),
		)
		.show(ctx, |ui| {
			ui.set_width(340.0);
			ui.heading("Crop profile picture");
			ui.add_space(12.0);
			let (preview, _) =
				ui.allocate_exact_size(egui::Vec2::splat(240.0), egui::Sense::hover());
			let width = crop.source.size[0] as f32;
			let height = crop.source.size[1] as f32;
			let side = (width.min(height) / crop.zoom.clamp(1.0, 3.0)).max(1.0);
			let left = (width - side) * (crop.pan.x.clamp(-1.0, 1.0) + 1.0) * 0.5;
			let top = (height - side) * (crop.pan.y.clamp(-1.0, 1.0) + 1.0) * 0.5;
			let image = egui::Rect::from_min_size(
				egui::pos2(
					preview.left() - preview.width() * left / side,
					preview.top() - preview.height() * top / side,
				),
				egui::vec2(
					preview.width() * width / side,
					preview.height() * height / side,
				),
			);
			ui.painter().with_clip_rect(preview).image(
				crop.texture.id(),
				image,
				egui::Rect::EVERYTHING,
				egui::Color32::WHITE,
			);
			ui.painter().circle_stroke(
				preview.center(),
				preview.width() * 0.5,
				egui::Stroke::new(2.0, egui::Color32::WHITE),
			);
			ui.add_space(8.0);
			ui.add(egui::Slider::new(&mut crop.zoom, 1.0..=3.0).text("Zoom"));
			ui.add(egui::Slider::new(&mut crop.pan.x, -1.0..=1.0).text("Horizontal"));
			ui.add(egui::Slider::new(&mut crop.pan.y, -1.0..=1.0).text("Vertical"));
			if let Some(error) = crop.error {
				design::notice(ui, design::Level::Error, error);
			}
			ui.add_space(8.0);
			ui.horizontal(|ui| {
				if design::button(ui, "Cancel", design::ButtonKind::Neutral).clicked() {
					decision = Some(CropDecision::Cancel);
				}
				if design::button(ui, "Use photo", design::ButtonKind::Primary).clicked() {
					decision = Some(CropDecision::Apply);
				}
			});
		});
	decision
}

fn encode_avatar_crop(crop: &CropDraft) -> Result<(String, egui::ColorImage), &'static str> {
	let width = crop.source.size[0] as u32;
	let height = crop.source.size[1] as u32;
	let side = ((width.min(height) as f32) / crop.zoom.clamp(1.0, 3.0))
		.round()
		.clamp(1.0, width.min(height) as f32) as u32;
	let x = (((width - side) as f32 * (crop.pan.x.clamp(-1.0, 1.0) + 1.0) * 0.5).round())
		.min((width - side) as f32) as u32;
	let y = (((height - side) as f32 * (crop.pan.y.clamp(-1.0, 1.0) + 1.0) * 0.5).round())
		.min((height - side) as f32) as u32;
	let bytes: Vec<_> = crop
		.source
		.pixels
		.iter()
		.flat_map(|pixel| pixel.to_srgba_unmultiplied())
		.collect();
	let source =
		image::RgbaImage::from_raw(width, height, bytes).ok_or("Could not prepare that crop")?;
	let square = image::imageops::crop_imm(&source, x, y, side, side).to_image();
	let resized = image::imageops::resize(&square, 256, 256, image::imageops::FilterType::Lanczos3);
	let mut png = std::io::Cursor::new(Vec::new());
	image::DynamicImage::ImageRgba8(resized.clone())
		.write_to(&mut png, image::ImageFormat::Png)
		.map_err(|_| "Could not encode that crop")?;
	let uri = discord_protocol::group_actions::icon_data_uri(png.get_ref())
		.ok_or("This crop is too large; adjust it and try again")?;
	let preview = egui::ColorImage::from_rgba_unmultiplied([256, 256], resized.as_raw());
	Ok((uri, preview))
}

fn form(
	ui: &mut egui::Ui,
	draft: &mut Draft,
	profile: &UserProfile,
	choosing: bool,
) -> Option<AvatarAction> {
	let colors = design::palette(ui);
	ui.spacing_mut().item_spacing.y = 6.0;
	let mut action = None;
	let has_picture = match &draft.avatar {
		Some(value) => value.is_some(),
		None => profile.user.avatar.is_some(),
	};
	design::row(
		ui,
		"profile-edit-form-profile-picture",
		Some(match &draft.avatar {
			Some(Some(_)) => "profile-edit-form-new-picture-chosen-save-to-upload-it",
			Some(None) => "profile-edit-form-your-picture-will-be-removed-when-you-save",
			None => "profile-edit-form-png-jpeg-gif-or-webp-up-to-8-mb-cropped",
		}),
		|ui| {
			ui.add_enabled_ui(!choosing, |ui| {
				if design::button(
					ui,
					&crate::i18n::translate("profile-edit-form-change"),
					design::ButtonKind::Outline,
				)
				.clicked()
				{
					action = Some(AvatarAction::Pick);
				}
			});
			if draft.avatar.is_some() {
				if design::text_action(ui, &crate::i18n::translate("profile-edit-form-undo"))
					.clicked()
				{
					action = Some(AvatarAction::Undo);
				}
			} else if has_picture
				&& design::text_action(ui, &crate::i18n::translate("profile-edit-form-remove"))
					.clicked()
			{
				action = Some(AvatarAction::Remove);
			}
			if choosing {
				ui.add(egui::Spinner::new().size(14.0));
			}
		},
	);
	ui.add_space(10.0);
	field(
		ui,
		"profile-edit-form-display-name",
		"profile-display-name",
		&mut draft.name,
		model::MAX_PROFILE_NAME_CHARS,
		false,
	);
	ui.label(
		egui::RichText::new(crate::i18n::translate(
			"profile-edit-form-leave-blank-to-use-your-username",
		))
		.size(12.0)
		.color(colors.muted),
	);
	ui.add_space(8.0);
	field(
		ui,
		"profile-edit-form-pronouns",
		"profile-pronouns",
		&mut draft.pronouns,
		model::MAX_PROFILE_PRONOUNS_CHARS,
		false,
	);
	ui.add_space(8.0);
	field(
		ui,
		"profile-edit-form-about-me",
		"profile-about-me",
		&mut draft.bio,
		model::MAX_PROFILE_BIO_CHARS,
		true,
	);
	ui.add_space(8.0);
	let mut enabled = draft.color.is_some();
	ui.vertical(|ui| {
		ui.spacing_mut().item_spacing.y = 3.0;
		ui.label(
			design::medium(
				ui,
				crate::i18n::translate("profile-edit-form-profile-color"),
				15.0,
			)
			.color(colors.text_strong),
		);
		ui.label(
			egui::RichText::new(crate::i18n::translate(
				"profile-edit-form-tints-your-banner-when-you-have-not-set-a-banner",
			))
			.size(13.0)
			.color(colors.muted),
		);
		ui.add_space(4.0);
		ui.horizontal(|ui| {
			if let Some(color) = &mut draft.color {
				let mut rgb = [(*color >> 16) as u8, (*color >> 8) as u8, *color as u8];
				if design::color_edit(ui, &mut rgb)
					.on_hover_text(crate::i18n::translate(
						"profile-edit-form-choose-profile-color",
					))
					.changed()
				{
					*color =
						(u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2]);
				}
				if design::text_action(ui, &crate::i18n::translate("profile-edit-form-use-default"))
					.clicked()
				{
					enabled = false;
				}
			} else if design::text_action(
				ui,
				&crate::i18n::translate("profile-edit-form-custom-color"),
			)
			.clicked()
			{
				enabled = true;
			}
		});
	});
	if enabled != draft.color.is_some() {
		// Start from the profile's own colors rather than the app accent.
		draft.color = enabled.then(|| {
			profile
				.accent_color
				.or(profile.theme_colors.map(|[top, _]| top))
				.unwrap_or(design::DEFAULT_PRIMARY_RGB)
		});
	}
	action
}

fn field(
	ui: &mut egui::Ui,
	label: &str,
	id: &str,
	value: &mut String,
	limit: usize,
	multiline: bool,
) {
	let colors = design::palette(ui);
	let label = crate::i18n::translate_if_key(label);
	let label_id = ui
		.horizontal(|ui| {
			let label = ui.label(design::eyebrow(ui, &label, colors.muted));
			if multiline {
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					ui.label(
						egui::RichText::new(format!("{} / {limit}", value.chars().count()))
							.size(11.0)
							.color(colors.muted),
					);
				});
			}
			label.id
		})
		.inner;
	let edit = if multiline {
		egui::TextEdit::multiline(value).desired_rows(4)
	} else {
		egui::TextEdit::singleline(value).align(egui::Align2::LEFT_CENTER)
	};
	design::input(
		ui,
		edit.id(egui::Id::unique(id))
			.char_limit(limit)
			.font(egui::FontId::proportional(15.0))
			.text_color(colors.text),
	)
	.labelled_by(label_id);
}

#[allow(clippy::too_many_arguments)]
fn preview(
	ui: &mut egui::Ui,
	draft: &Draft,
	profile: &UserProfile,
	avatars: &mut Avatars,
	media: (bool, &[model::Guild]),
	opening: &mut Option<String>,
	picture: Picture<'_>,
	clickable: bool,
) -> Option<AvatarAction> {
	let (demo, guilds) = media;
	let colors = design::palette(ui);
	let mut action = None;
	ui.label(design::eyebrow(
		ui,
		crate::i18n::translate("profile-edit-preview-preview"),
		colors.muted,
	));
	ui.add_space(4.0);
	// Paint with the profile's own colors, as other people see the card, not the app theme.
	let theme = crate::profiles::Theme::new(&colors, profile.theme_colors);
	let rgb = |c: u32| egui::Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
	ui.scope(|ui| {
		let background = ui.painter().add(egui::Shape::Noop);
		ui.visuals_mut().override_text_color = Some(theme.text);
		ui.visuals_mut().hyperlink_color = theme.link;
		ui.set_width(ui.available_width());
		ui.spacing_mut().item_spacing.y = 0.0;
		{
			let (banner, _) = ui
				.allocate_exact_size(egui::vec2(ui.available_width(), 90.0), egui::Sense::hover());
			let corner = egui::CornerRadius {
				nw: 12,
				ne: 12,
				sw: 0,
				se: 0,
			};
			if profile.banner.is_some() {
				// The banner image covers its color, so the saved profile paints it as is.
				avatars.paint_banner(ui, profile, banner, corner, demo);
			} else {
				let color = profile
					.theme_colors
					.map(crate::profiles::Theme::banner)
					.or_else(|| draft.color.map(rgb))
					.unwrap_or(colors.raised);
				ui.painter().rect_filled(banner, corner, color);
			}
			let avatar = egui::Rect::from_min_size(
				banner.left_bottom() + egui::vec2(16.0, -40.0),
				egui::Vec2::splat(80.0),
			);
			ui.painter()
				.circle_filled(avatar.center(), 46.0, theme.card);
			let response =
				ui.scope_builder(
					egui::UiBuilder::new().max_rect(avatar),
					|ui| match picture {
						Picture::Pending(texture) | Picture::Saved(texture) => ui.add(
							egui::Image::new(texture)
								.fit_to_exact_size(egui::Vec2::splat(80.0))
								.corner_radius(40.0),
						),
						Picture::Removed => design::avatar(ui, &profile.user.name, 80.0),
						Picture::Remote => avatars.with_avatar_animation(true, |avatars| {
							avatars.show_plain(ui, &profile.user, 80.0, demo)
						}),
					},
				);
			if clickable {
				let hit = ui.interact(
					avatar,
					ui.scope_id().with("change-avatar"),
					egui::Sense::click(),
				);
				let hit = hit.on_hover_text(crate::i18n::translate(
					"profile-edit-preview-change-profile-picture",
				));
				if hit.hovered() || hit.has_focus() {
					ui.painter().circle_filled(
						avatar.center(),
						40.0,
						egui::Color32::from_black_alpha(120),
					);
					icons::paint(
						ui.painter(),
						icons::Icon::Pencil,
						egui::Rect::from_center_size(avatar.center(), egui::Vec2::splat(22.0)),
						egui::Color32::WHITE,
					);
				}
				if hit.clicked() {
					action = Some(AvatarAction::Pick);
				}
			}
			drop(response);
			ui.add_space((avatar.bottom() + 12.0 - ui.cursor().top()).max(0.0));
			egui::Frame::new()
				.inner_margin(egui::Margin {
					left: 12,
					right: 12,
					top: 0,
					bottom: 12,
				})
				.show(ui, |ui| {
					egui::Frame::new()
						.fill(theme.panel)
						.corner_radius(8)
						.inner_margin(12)
						.show(ui, |ui| {
							ui.set_width(ui.available_width());
							ui.set_min_height(138.0);
							ui.spacing_mut().item_spacing.y = 4.0;
							let name = if draft.name.is_empty() {
								&profile.username
							} else {
								&draft.name
							};
							ui.add(
								egui::Label::new(
									design::semibold(ui, name, 20.0).color(theme.text),
								)
								.wrap(),
							);
							ui.label(
								egui::RichText::new(&profile.username)
									.size(13.0)
									.color(theme.text),
							);
							if !draft.pronouns.is_empty() {
								ui.label(
									egui::RichText::new(&draft.pronouns)
										.size(12.0)
										.color(theme.muted),
								);
							}
							if !draft.bio.is_empty() {
								ui.add_space(8.0);
								ui.separator();
								ui.add_space(8.0);
								ui.label(design::eyebrow(
									ui,
									crate::i18n::translate("profile-edit-preview-about-me"),
									theme.text,
								));
								let mut mentions = crate::profiles::ProfileSession::default();
								crate::markdown::Formatted::parse(&draft.bio).show_with_images(
									ui,
									opening,
									&[],
									None,
									&mut mentions,
									(avatars, demo, guilds),
									crate::design::MessageCardSurface::Opaque,
								);
							}
						});
				});
		}
		let rect = ui.min_rect();
		ui.painter().set(background, theme.background(rect));
	});
	action
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn draft_only_sends_changed_fields_and_can_clear_values() {
		let user = model::User {
			id: model::Id(1),
			name: "Synthetic".into(),
			avatar: None,
			webhook: false,
			kind: Default::default(),
			discriminator: 0,
			primary_guild: None,
			cosmetics: None,
		};
		let mut profile = crate::profiles::synthetic(&user, None);
		profile.global_name = Some("Before".into());
		profile.accent_color = Some(0x123456);
		let mut draft = Draft::from_profile(&profile);
		assert!(draft.changes(&profile) == ProfileEdit::default());
		draft.name.clear();
		draft.bio.clear();
		draft.color = None;
		let changes = draft.changes(&profile);
		assert_eq!(changes.global_name, Some(None));
		assert_eq!(changes.bio, Some(String::new()));
		assert_eq!(changes.accent_color, Some(None));
		assert_eq!(changes.pronouns, None);
		assert!(changes.valid());
		// A reload retains edited values but adopts remote changes to untouched fields.
		let before = Draft::from_profile(&profile);
		profile.pronouns = "she/her".into();
		draft.rebase(&before, &Draft::from_profile(&profile));
		assert_eq!(draft.pronouns, "she/her");
		assert!(draft.name.is_empty() && draft.bio.is_empty());
		assert_eq!(draft.changes(&profile).pronouns, None);
	}
	#[test]
	fn editor_saves_once_preserves_failed_draft_and_cancels_in_both_themes() {
		fn frame(
			ctx: &egui::Context,
			editor: &mut Editor,
			state: &mut State,
			avatars: &mut Avatars,
			width: f32,
			events: Vec<egui::Event>,
		) -> (Vec<Command>, Vec<(String, egui::Rect)>) {
			let mut commands = vec![];
			let mut output = ctx.run_ui(
				egui::RawInput {
					screen_rect: Some(egui::Rect::from_min_size(
						egui::Pos2::ZERO,
						egui::vec2(width, 1100.0),
					)),
					events,
					..Default::default()
				},
				|ui| editor.show(ui, state, avatars, &mut commands),
			);
			output.textures_delta.clear();
			fn text(shape: &egui::Shape, labels: &mut Vec<(String, egui::Rect)>) {
				match shape {
					egui::Shape::Text(text) => labels.push((
						text.galley.job.text.clone(),
						text.galley.rect.translate(text.pos.to_vec2()),
					)),
					egui::Shape::Vec(shapes) => {
						for shape in shapes {
							text(shape, labels);
						}
					}
					_ => {}
				}
			}
			let mut labels = vec![];
			for shape in output.shapes {
				text(&shape.shape, &mut labels);
			}
			(commands, labels)
		}
		for theme in [egui::ThemePreference::Dark, egui::ThemePreference::Light] {
			for width in [320.0, 720.0] {
				let ctx = egui::Context::default();
				ctx.set_theme(theme);
				let mut editor = Editor::default();
				let mut avatars = Avatars::default();
				let mut state = test_support::demo_state();
				let (commands, _) =
					frame(&ctx, &mut editor, &mut state, &mut avatars, width, vec![]);
				assert_eq!(commands.len(), 1);
				let Command::EditProfile {
					user,
					request,
					changes: None,
				} = commands.into_iter().next().unwrap()
				else {
					panic!("load expected")
				};
				state.apply(client_core::Envelope {
					generation: state.generation,
					event: client_core::Event::ProfileEdited {
						user,
						request,
						result: Ok(Box::new(crate::profiles::synthetic(
							state.user.as_ref().unwrap(),
							None,
						))),
					},
				});
				frame(&ctx, &mut editor, &mut state, &mut avatars, width, vec![]);
				ctx.memory_mut(|memory| {
					memory.request_focus(egui::Id::unique("profile-display-name"))
				});
				frame(
					&ctx,
					&mut editor,
					&mut state,
					&mut avatars,
					width,
					vec![egui::Event::Text(" Edited".into())],
				);
				let draft = editor.draft.as_ref().unwrap().clone();
				assert!(draft.name.contains("Edited"));
				let (_, labels) = frame(&ctx, &mut editor, &mut state, &mut avatars, width, vec![]);
				let save = labels
					.iter()
					.find(|(text, _)| text == "Save changes")
					.unwrap()
					.1;
				assert!(save.left() >= 0.0 && save.right() <= width);
				let mut sent = vec![];
				for pressed in [true, false] {
					let pos = save.center();
					let (commands, _) = frame(
						&ctx,
						&mut editor,
						&mut state,
						&mut avatars,
						width,
						vec![
							egui::Event::PointerMoved(pos),
							egui::Event::PointerButton {
								pos,
								button: egui::PointerButton::Primary,
								pressed,
								modifiers: egui::Modifiers::NONE,
							},
						],
					);
					sent.extend(commands);
				}
				assert_eq!(sent.len(), 1);
				assert!(
					frame(&ctx, &mut editor, &mut state, &mut avatars, width, vec![])
						.0
						.is_empty()
				);
				let Command::EditProfile {
					user,
					request,
					changes: Some(changes),
				} = sent.pop().unwrap()
				else {
					panic!("save expected")
				};
				assert_eq!(changes.global_name, Some(Some(draft.name.clone())));
				state.apply(client_core::Envelope {
					generation: state.generation,
					event: client_core::Event::ProfileEdited {
						user,
						request,
						result: Err(client_core::auth::Failure::Ambiguous),
					},
				});
				let (_, labels) = frame(&ctx, &mut editor, &mut state, &mut avatars, width, vec![]);
				assert!(editor.draft.as_ref() == Some(&draft));
				assert!(!state.can_save_own_profile());
				let cancel = labels
					.iter()
					.find(|(text, _)| text == "Cancel")
					.unwrap()
					.1
					.center();
				for pressed in [true, false] {
					assert!(
						frame(
							&ctx,
							&mut editor,
							&mut state,
							&mut avatars,
							width,
							vec![
								egui::Event::PointerMoved(cancel),
								egui::Event::PointerButton {
									pos: cancel,
									button: egui::PointerButton::Primary,
									pressed,
									modifiers: egui::Modifiers::NONE
								}
							]
						)
						.0
						.is_empty()
					);
				}
				assert!(
					editor.draft.as_ref()
						== Some(&Draft::from_profile(
							state.own_profile.data.as_ref().unwrap()
						))
				);
			}
		}
	}
}
