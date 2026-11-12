//! Bounded decoder for the user-supplied, unofficial embedded-application search sample.
use crate::DecodeError;
use model::{Id, application_search as app};
use serde::Deserialize;

#[derive(Deserialize)]
struct SearchResponse {
	#[serde(default)]
	results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
	#[serde(rename = "type")]
	kind: u8,
	data: Application,
}

#[derive(Deserialize)]
struct Application {
	id: Id,
	name: String,
	#[serde(default)]
	icon: Option<String>,
	#[serde(default)]
	description: String,
	#[serde(default)]
	is_monetized: bool,
	#[serde(default)]
	is_verified: bool,
	#[serde(default)]
	is_discoverable: bool,
	#[serde(default)]
	tags: Vec<String>,
	#[serde(default)]
	categories: Vec<Category>,
	#[serde(default)]
	embedded_activity_config: Option<ActivityConfig>,
}

#[derive(Deserialize)]
struct Category {
	id: u16,
	name: String,
}

#[derive(Deserialize)]
struct ActivityConfig {
	#[serde(default)]
	supported_platforms: Vec<String>,
	#[serde(default)]
	release_phase: Option<String>,
}

/// Parses only embedded-activity search results; unknown fields such as `embedded_surfaces`
/// are ignored. Discord's user-facing search API is undocumented and can change without notice.
pub fn parse(bytes: &[u8]) -> Result<Vec<app::Application>, DecodeError> {
	if bytes.len() > 256 * 1024 {
		return Err(DecodeError);
	}
	let response: SearchResponse = serde_json::from_slice(bytes).map_err(|_| DecodeError)?;
	if response.results.len() > app::MAX_RESULTS * 2 {
		return Err(DecodeError);
	}
	let mut entries = Vec::new();
	for result in response
		.results
		.into_iter()
		.filter(|result| result.kind == 1)
	{
		let item = app::Application {
			id: result.data.id,
			name: result.data.name,
			icon: result.data.icon,
			description: result.data.description,
			verified: result.data.is_verified,
			discoverable: result.data.is_discoverable,
			monetized: result.data.is_monetized,
			tags: result.data.tags,
			categories: result
				.data
				.categories
				.into_iter()
				.map(|category| app::Category {
					id: category.id,
					name: category.name,
				})
				.collect(),
			activity: result
				.data
				.embedded_activity_config
				.map(|activity| app::ActivityConfig {
					supported_platforms: activity.supported_platforms,
					release_phase: activity.release_phase,
				}),
		};
		if item.valid() {
			entries.push(item);
		}
		if entries.len() == app::MAX_RESULTS {
			break;
		}
	}
	if entries.iter().map(app::Application::bytes).sum::<usize>() > app::MAX_CATALOG_BYTES {
		return Err(DecodeError);
	}
	Ok(entries)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn supplied_search_shape_decodes_and_ignores_unknown_fields() {
		let bytes = br#"{"results":[{"type":1,"data":{"id":"1211781489931452447","name":"Wordle","icon":"8904fec86de9bb7a71b2ae1e71a1adbc","description":"Play Wordle","is_monetized":false,"is_verified":true,"is_discoverable":true,"embedded_surfaces":[1],"embedded_activity_config":{"supported_platforms":["web","ios","android"],"release_phase":"global_launch"},"tags":["Games"],"categories":[{"id":6,"name":"Games"}]}}]}"#;
		let entries = parse(bytes).unwrap();
		assert_eq!(entries.len(), 1);
		assert_eq!(entries[0].name, "Wordle");
		assert_eq!(
			entries[0].activity.as_ref().unwrap().supported_platforms,
			["web", "ios", "android"]
		);
	}

	#[test]
	fn invalid_ids_and_oversized_response_are_rejected_or_skipped() {
		let invalid = br#"{"results":[{"type":1,"data":{"id":"0","name":"bad"}}]}"#;
		assert!(parse(invalid).is_err());
		assert!(parse(&vec![b' '; 256 * 1024 + 1]).is_err());
	}
}
