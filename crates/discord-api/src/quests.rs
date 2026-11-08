//! The account's current Quests response, normalized into bounded text and CDN art.
use client_core::auth::Failure;
use model::Quest;
use serde_json::Value;

const MAX_QUESTS: usize = 100;

fn field<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
	path.iter().try_fold(value, |value, key| value.get(*key))
}

fn text(value: &Value, paths: &[&[&str]], max: usize) -> String {
	paths
		.iter()
		.find_map(|path| field(value, path).and_then(Value::as_str))
		.map(|text| text.chars().take(max).collect())
		.unwrap_or_default()
}

pub(super) fn parse(bytes: &[u8]) -> Result<Vec<Quest>, Failure> {
	let root: Value = serde_json::from_slice(bytes).map_err(|_| Failure::Protocol)?;
	let rows = root
		.get("user_quests")
		.and_then(Value::as_array)
		.or_else(|| root.get("quests").and_then(Value::as_array))
		.ok_or(Failure::Protocol)?;
	if rows.len() > MAX_QUESTS {
		return Err(Failure::Capacity);
	}
	let mut quests = Vec::with_capacity(rows.len());
	for row in rows {
		let source = row.get("quest").unwrap_or(row);
		let config = source.get("config").unwrap_or(source);
		let id = text(row, &[&["quest_id"], &["quest", "id"], &["id"]], 64);
		if row
			.get("user_status")
			.and_then(|status| status.get("completed_at"))
			.is_some_and(|completed| !completed.is_null())
		{
			continue;
		}
		let title = text(
			source,
			&[
				&["config", "messages", "quest_name"],
				&["config", "messages", "name"],
				&["config", "application", "name"],
				&["messages", "quest_name"],
				&["application", "name"],
				&["name"],
			],
			256,
		);
		let description = text(
			source,
			&[
				&["config", "messages", "quest_description"],
				&["config", "messages", "description"],
				&["messages", "quest_description"],
				&["description"],
			],
			1024,
		);
		let image = text(
			config,
			&[
				&["assets", "hero"],
				&["assets", "logo"],
				&["application", "icon_url"],
				&["application", "icon"],
			],
			2048,
		);
		let image = (image.starts_with("https://cdn.discordapp.com/")
			|| image.starts_with("https://media.discordapp.net/"))
		.then_some(image);
		let progress = quest_progress(row, config);
		let quest = Quest {
			id,
			title,
			description,
			progress,
			image,
		};
		if quest.valid() {
			quests.push(quest);
		}
	}
	Ok(quests)
}

fn quest_progress(row: &Value, config: &Value) -> String {
	let Some(progress) = field(row, &["user_status", "progress"]).and_then(Value::as_object) else {
		return "In progress".into();
	};
	for (task, status) in progress {
		let current = status
			.get("value")
			.and_then(Value::as_f64)
			.or_else(|| status.as_f64());
		let target =
			field(config, &["task_config", "tasks", task, "target"]).and_then(Value::as_f64);
		if let (Some(current), Some(target)) = (current, target)
			&& current.is_finite()
			&& target.is_finite()
			&& target > 0.0
		{
			let percent = (current / target * 100.0).clamp(0.0, 100.0).round() as u32;
			return format!("{} / {} · {percent}%", current.round(), target.round());
		}
	}
	"In progress".into()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalizes_only_bounded_text_and_known_cdn_images() {
		let rows = parse(
			br#"{"user_quests":[{"quest":{"id":"123","config":{"messages":{"quest_name":"Play a game","quest_description":"Launch the game"},"task_config":{"tasks":{"WATCH_VIDEO":{"target":20}}},"assets":{"hero":"https://cdn.discordapp.com/quests/hero.png"}}},"user_status":{"progress":{"WATCH_VIDEO":{"value":5}}}},{"quest_id":"124","config":{"application":{"name":"A quest","icon":"https://evil.invalid/image.png"}}},{"quest":{"id":"125","config":{"messages":{"quest_name":"Finished quest"}}},"user_status":{"completed_at":"2026-11-08T00:00:00.000000+00:00"}}]}"#,
		)
	.unwrap();
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0].title, "Play a game");
		assert_eq!(rows[0].progress, "5 / 20 · 25%");
		assert_eq!(
			rows[0].image.as_deref(),
			Some("https://cdn.discordapp.com/quests/hero.png")
		);
		assert_eq!(rows[1].title, "A quest");
		assert_eq!(rows[1].image, None);
		assert!(!rows.iter().any(|quest| quest.title == "Finished quest"));
		assert!(parse(br#"{"user_quests":[{}]}"#).unwrap().is_empty());
		assert!(parse(br#"{"user_quests":[]}"#).is_ok());
	}

	#[test]
	fn rejects_unknown_response_shape_and_excessive_quest_count() {
		assert!(parse(br#"{}"#).is_err());
		let rows = vec![serde_json::json!({}); MAX_QUESTS + 1];
		let bytes = serde_json::to_vec(&serde_json::json!({"user_quests": rows})).unwrap();
		assert!(matches!(parse(&bytes), Err(Failure::Capacity)));
	}
}
