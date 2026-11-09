//! Fetch Discord's public display-name font assets only when a visible styled name needs one.
//! These requests are unauthenticated and remain separate from the account API client.

use std::{sync::OnceLock, time::Duration};

use egui::Context;
use futures_util::StreamExt;

const MAX_WOFF2_BYTES: usize = 320 * 1024;
const MAX_SFNT_BYTES: usize = 32 * 1024 * 1024;

static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
static SLOTS: OnceLock<std::sync::Arc<tokio::sync::Semaphore>> = OnceLock::new();

fn client() -> Option<reqwest::Client> {
	CLIENT
		.get_or_init(|| {
			reqwest::Client::builder()
				.https_only(true)
				.redirect(reqwest::redirect::Policy::none())
				.connect_timeout(Duration::from_secs(3))
				.timeout(Duration::from_secs(8))
				.build()
				.ok()
		})
		.clone()
}

fn slots() -> std::sync::Arc<tokio::sync::Semaphore> {
	SLOTS
		.get_or_init(|| std::sync::Arc::new(tokio::sync::Semaphore::new(2)))
		.clone()
}

pub(crate) fn load_requested(runtime: &tokio::runtime::Runtime, ctx: &Context) {
	let assets = ui::fonts::take_name_font_requests();
	if assets.is_empty() {
		return;
	}
	let Some(client) = client() else {
		return;
	};
	let slots = slots();
	for asset in assets {
		let client = client.clone();
		let slots = slots.clone();
		let ctx = ctx.clone();
		runtime.spawn(async move {
			let Ok(_permit) = slots.acquire_owned().await else {
				return;
			};
			let Some(compressed) = download(&client, asset.url).await else {
				return;
			};
			let decoded = tokio::task::spawn_blocking(move || decode_woff2(&compressed))
				.await
				.ok()
				.flatten();
			if let Some(decoded) = decoded {
				ui::fonts::install_name_font(&ctx, asset.id, decoded);
			}
		});
	}
}

async fn download(client: &reqwest::Client, url: &'static str) -> Option<Vec<u8>> {
	if !valid_asset_url(url) {
		return None;
	}
	let response = client.get(url).send().await.ok()?;
	if !response.status().is_success()
		|| response
			.headers()
			.get(reqwest::header::CONTENT_TYPE)?
			.to_str()
			.ok()?
			.split(';')
			.next()?
			.trim() != "font/woff2"
		|| response
			.content_length()
			.is_some_and(|length| length > MAX_WOFF2_BYTES as u64)
	{
		return None;
	}
	let mut stream = response.bytes_stream();
	let mut bytes = Vec::new();
	while let Some(chunk) = stream.next().await {
		let chunk = chunk.ok()?;
		if bytes.len().saturating_add(chunk.len()) > MAX_WOFF2_BYTES {
			return None;
		}
		bytes.extend_from_slice(&chunk);
	}
	Some(bytes)
}

fn valid_asset_url(raw: &str) -> bool {
	let Ok(url) = url::Url::parse(raw) else {
		return false;
	};
	url.scheme() == "https"
		&& url.host_str() == Some("discord.com")
		&& url.port().is_none()
		&& url.username().is_empty()
		&& url.password().is_none()
		&& url.query().is_none()
		&& url.fragment().is_none()
		&& url.path().starts_with("/assets/")
		&& url.path().ends_with(".woff2")
}

fn decode_woff2(compressed: &[u8]) -> Option<Vec<u8>> {
	const HEADER_LEN: usize = 48;
	if compressed.len() < HEADER_LEN
		|| compressed.len() > MAX_WOFF2_BYTES
		|| &compressed[..4] != b"wOF2"
		|| usize::try_from(u32::from_be_bytes(compressed[8..12].try_into().ok()?)).ok()?
			!= compressed.len()
		|| usize::try_from(u32::from_be_bytes(compressed[16..20].try_into().ok()?)).ok()?
			> MAX_SFNT_BYTES
	{
		return None;
	}
	let decoded = wuff::decompress_woff2(compressed).ok()?;
	(decoded.len() <= MAX_SFNT_BYTES).then_some(decoded)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn font_asset_requests_are_limited_to_public_discord_woff2_paths() {
		for url in [
			"https://discord.com/assets/33d4f12a85e1f736.woff2",
			"https://discord.com/assets/asset.woff2",
		] {
			assert!(valid_asset_url(url));
		}
		for url in [
			"http://discord.com/assets/font.woff2",
			"https://discord.com.evil.test/assets/font.woff2",
			"https://user@discord.com/assets/font.woff2",
			"https://discord.com/assets/font.woff2?token=synthetic",
			"https://cdn.discordapp.com/assets/font.woff2",
		] {
			assert!(!valid_asset_url(url), "accepted {url}");
		}
	}

	#[test]
	fn decoder_rejects_bad_or_oversized_woff2_before_decompression() {
		assert!(decode_woff2(b"synthetic invalid font").is_none());
		let mut oversized = vec![0; 48];
		oversized[..4].copy_from_slice(b"wOF2");
		oversized[8..12].copy_from_slice(&48u32.to_be_bytes());
		oversized[16..20]
			.copy_from_slice(&u32::try_from(MAX_SFNT_BYTES + 1).unwrap().to_be_bytes());
		assert!(decode_woff2(&oversized).is_none());
	}
}
