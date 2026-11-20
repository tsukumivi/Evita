//! One compact, theme-aware account name/badge row for chat and people.
use crate::{design, icons};
use egui::{Color32, Response, Sense, Ui, vec2};
use model::User;

#[allow(clippy::too_many_arguments)]
pub(super) fn name(
	ui: &mut Ui,
	user: &User,
	name: &str,
	size: f32,
	color: (Color32, Option<model::server_roles::Colors>, Color32),
	sense: Sense,
	trailing: f32,
	avatars: &mut crate::avatars::Avatars,
	demo: bool,
) -> Response {
	name_with_mode(
		ui, user, name, size, color, sense, trailing, false, avatars, demo,
	)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn server_member_name(
	ui: &mut Ui,
	user: &User,
	name: &str,
	size: f32,
	color: (Color32, Option<model::server_roles::Colors>, Color32),
	sense: Sense,
	trailing: f32,
	avatars: &mut crate::avatars::Avatars,
	demo: bool,
) -> Response {
	name_with_mode(
		ui, user, name, size, color, sense, trailing, true, avatars, demo,
	)
}

#[allow(clippy::too_many_arguments)]
fn name_with_mode(
	ui: &mut Ui,
	user: &User,
	name: &str,
	size: f32,
	color: (Color32, Option<model::server_roles::Colors>, Color32),
	sense: Sense,
	trailing: f32,
	server_member: bool,
	avatars: &mut crate::avatars::Avatars,
	demo: bool,
) -> Response {
	let colors = design::palette(ui);
	let account_label = user.account_label();
	let verified = user.kind == model::AccountKind::VerifiedBot;
	let badge = account_label.map(|text| {
		let key = match text {
			"BOT" => "account-badge-bot",
			"APP" => "account-badge-app",
			_ => "account-badge-webhook",
		};
		ui.painter().layout_no_wrap(
			crate::i18n::translate_if_key(key),
			egui::FontId::proportional(10.0),
			colors.accent_text,
		)
	});
	let bot_reserve = badge.as_ref().map_or(0.0, |text| {
		text.size().x + 8.0 + if verified { 10.0 } else { 0.0 } + ui.spacing().item_spacing.x
	});
	let server_tag = user.primary_guild.as_deref();
	let tag_reserve = crate::profiles::server_tag_width(ui, server_tag)
		+ if server_tag.is_some() {
			ui.spacing().item_spacing.x
		} else {
			0.0
		};
	let width = (ui.available_width() - bot_reserve - tag_reserve - trailing).max(0.0);
	// In DMs/messages, a Nitro style replaces the role color; server rows use role colors instead.
	let response = ui
		.scope(|ui| {
			ui.set_max_width(width);
			let style = crate::name_styles::style(user);
			let font = egui::FontId::new(size, design::medium_family(ui.ctx()));
			if server_member {
				crate::name_styles::server_member_label(
					ui, style, name, font, color.1, color.2, color.0, sense,
				)
			} else {
				crate::name_styles::label(
					ui, style, name, font, color.1, color.2, color.0, sense, false,
				)
			}
		})
		.inner;
	if let Some(text) = badge {
		let verified_width = if verified { 10.0 } else { 0.0 };
		let (rect, badge) = ui.allocate_exact_size(
			vec2(text.size().x + 8.0 + verified_width, 16.0),
			Sense::hover(),
		);
		ui.painter().rect_filled(rect, 3, colors.accent);
		if verified {
			icons::paint(
				ui.painter(),
				icons::Icon::Check,
				egui::Rect::from_center_size(
					egui::pos2(rect.left() + 8.5, rect.center().y),
					vec2(9.0, 9.0),
				),
				colors.accent_text,
			);
		}
		ui.painter().galley(
			egui::pos2(
				rect.left() + 4.0 + verified_width,
				rect.center().y - text.size().y * 0.5,
			),
			text,
			colors.accent_text,
		);
		let description_key = match (account_label, verified) {
			(_, true) => "account-badge-verified-bot-description",
			(Some("BOT"), _) => "account-badge-bot-description",
			(Some("APP"), _) => "account-badge-app-description",
			_ => "account-badge-webhook-description",
		};
		let description = crate::i18n::translate_if_key(description_key);
		badge.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &description));
		badge.on_hover_text(description);
	}
	if let Some(tag) = server_tag {
		crate::profiles::server_tag(ui, tag, avatars, demo);
	}
	response
}
