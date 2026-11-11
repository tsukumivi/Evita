//! One bounded, cancellable local sound preparation job; selection never uploads by itself.
use model::Id;
use std::{
	io::Read,
	path::PathBuf,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
		mpsc,
	},
};

type Scope = (u64, Id, u64);
type Prepared = (String, String, Vec<u8>, u16);
type Selected = Result<Option<Prepared>, &'static str>;

struct Choosing {
	scope: Scope,
	result: mpsc::Receiver<Selected>,
	cancelled: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct SoundboardUpload {
	choosing: Option<Choosing>,
}

impl SoundboardUpload {
	pub fn start(
		&mut self,
		scope: Scope,
		runtime: &tokio::runtime::Handle,
		context: &egui::Context,
		parent: Arc<winit::window::Window>,
	) -> Result<(), &'static str> {
		if self.choosing.is_some() {
			return Err("Close the previous sound picker first");
		}
		let dialog = platform::save::soundboard_source(parent);
		let (send, result) = mpsc::sync_channel(1);
		let cancelled = Arc::new(AtomicBool::new(false));
		let flag = cancelled.clone();
		let context = context.clone();
		runtime.spawn(async move {
			let result = match dialog.await {
				Some(path) if !flag.load(Ordering::Acquire) => {
					let flag = flag.clone();
					tokio::task::spawn_blocking(move || read(path, &flag).map(Some))
						.await
						.unwrap_or(Err("Sound preparation interrupted; choose it again"))
				}
				_ => Ok(None),
			};
			let _ = send.send(if flag.load(Ordering::Acquire) {
				Ok(None)
			} else {
				result
			});
			context.request_repaint();
		});
		self.choosing = Some(Choosing {
			scope,
			result,
			cancelled,
		});
		Ok(())
	}

	pub fn cancel(&self) {
		if let Some(job) = &self.choosing {
			job.cancelled.store(true, Ordering::Release);
		}
	}

	pub fn poll(
		&mut self,
		generation: u64,
		valid: impl FnOnce(Id) -> bool,
	) -> Option<(Scope, Selected)> {
		let job = self.choosing.as_ref()?;
		if job.scope.0 != generation || !valid(job.scope.1) {
			self.cancel();
		}
		let result = match job.result.try_recv() {
			Ok(result) => result,
			Err(mpsc::TryRecvError::Empty) => return None,
			Err(mpsc::TryRecvError::Disconnected) => {
				Err("Sound preparation interrupted; choose it again")
			}
		};
		let job = self.choosing.take()?;
		(!job.cancelled.load(Ordering::Acquire)).then_some((job.scope, result))
	}
}

impl Drop for SoundboardUpload {
	fn drop(&mut self) {
		self.cancel();
	}
}

fn read(path: PathBuf, cancelled: &AtomicBool) -> Result<Prepared, &'static str> {
	const MAX_INPUT: u64 = 512 * 1024;
	const MAX_DURATION_MS: u64 = 5200;
	if !path.is_absolute() || path.as_os_str().as_encoded_bytes().len() > 4096 {
		return Err("Choose an MP3 or Ogg file with a supported local path");
	}
	let metadata =
		std::fs::symlink_metadata(&path).map_err(|_| "Could not open the chosen sound")?;
	if !metadata.is_file()
		|| metadata.file_type().is_symlink()
		|| metadata.len() == 0
		|| metadata.len() > MAX_INPUT
	{
		return Err("Choose a regular MP3 or Ogg file up to 512 KB");
	}
	let mut bytes = Vec::with_capacity(metadata.len() as usize);
	std::fs::File::open(&path)
		.map_err(|_| "Could not open the chosen sound")?
		.take(MAX_INPUT + 1)
		.read_to_end(&mut bytes)
		.map_err(|_| "Could not read the chosen sound")?;
	if bytes.len() as u64 > MAX_INPUT || cancelled.load(Ordering::Acquire) {
		return Err("Sound preparation cancelled or exceeds 512 KB");
	}
	let extension = path
		.extension()
		.and_then(|value| value.to_str())
		.map(str::to_ascii_lowercase)
		.ok_or("Choose an MP3 or Ogg file")?;
	let content_type = match extension.as_str() {
		"mp3" if bytes.starts_with(b"ID3") || bytes.first().is_some_and(|b| *b == 0xff) => {
			"audio/mpeg"
		}
		"ogg" if bytes.starts_with(b"OggS") => "audio/ogg",
		_ => return Err("The chosen file does not match its MP3 or Ogg extension"),
	};
	let duration_ms = duration_ms(&bytes, &extension, cancelled)?;
	if duration_ms == 0 || u64::from(duration_ms) > MAX_DURATION_MS {
		return Err("Soundboard sounds can be at most 5.2 seconds long");
	}
	let mut name: String = path
		.file_stem()
		.and_then(|name| name.to_str())
		.unwrap_or("sound")
		.chars()
		.filter(|character| !character.is_control())
		.take(32)
		.collect();
	name = name.trim().to_owned();
	if name.chars().count() < 2 {
		name = "Sound".into();
	}
	Ok((name, content_type.into(), bytes, duration_ms))
}

fn duration_ms(bytes: &[u8], extension: &str, cancelled: &AtomicBool) -> Result<u16, &'static str> {
	use symphonia::core::{
		codecs::{CODEC_ID_MP3, CODEC_ID_OPUS, CODEC_ID_VORBIS},
		formats::{FormatOptions, TrackType},
		io::{MediaSourceStream, MediaSourceStreamOptions},
		meta::{Limit, MetadataOptions},
		probe::Hint,
	};
	let source = std::io::Cursor::new(bytes.to_vec());
	let source = MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());
	let mut hint = Hint::new();
	hint.with_extension(extension);
	let metadata = MetadataOptions::default()
		.limit_tag_bytes(Limit::Maximum(0))
		.limit_visual_bytes(Limit::Maximum(0));
	let mut format = symphonia::default::get_probe()
		.probe(&hint, source, FormatOptions::default(), metadata)
		.map_err(|_| "The chosen file is not valid MP3 or Ogg audio")?;
	let track = format
		.default_track(TrackType::Audio)
		.ok_or("The chosen file contains no audio track")?;
	let parameters = track
		.codec_params
		.as_ref()
		.ok_or("The chosen file contains no readable audio track")?;
	if !matches!(
		parameters.codec,
		CODEC_ID_MP3 | CODEC_ID_OPUS | CODEC_ID_VORBIS
	) {
		return Err("Use MP3 audio or Ogg with Opus/Vorbis audio");
	}
	if !parameters
		.sample_rate
		.is_some_and(|rate| (8_000..=96_000).contains(&rate))
	{
		return Err("The sound has an unsupported sample rate");
	}
	let track_id = track.id;
	let time_base = track
		.time_base
		.ok_or("The chosen audio duration could not be verified")?;
	let mut duration_nanos = 0i128;
	let mut packets = 0usize;
	while !cancelled.load(Ordering::Acquire) {
		let packet = match format.next_packet() {
			Ok(Some(packet)) => packet,
			Ok(None) => break,
			Err(_) => return Err("The chosen audio is incomplete or invalid"),
		};
		packets += 1;
		if packets > 100_000 {
			return Err("The chosen audio contains too many packets");
		}
		if packet.track_id == track_id {
			let nanos = time_base
				.calc_duration(packet.dur)
				.ok_or("The chosen audio duration is invalid")?
				.as_nanos();
			duration_nanos = duration_nanos
				.checked_add(nanos)
				.ok_or("The chosen audio duration is invalid")?;
			if duration_nanos > 5_200_000_000 {
				return Err("Soundboard sounds can be at most 5.2 seconds long");
			}
		}
	}
	if cancelled.load(Ordering::Acquire) {
		return Err("Sound preparation cancelled");
	}
	let millis = u64::try_from(duration_nanos.div_ceil(1_000_000))
		.map_err(|_| "The chosen audio duration is invalid")?;
	u16::try_from(millis).map_err(|_| "Soundboard sounds can be at most 5.2 seconds long")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn malformed_or_mismatched_audio_is_rejected_before_upload() {
		let root = std::env::temp_dir().join("evita-soundboard-upload-test.mp3");
		std::fs::write(&root, b"not an mp3").unwrap();
		assert!(read(root.clone(), &AtomicBool::new(false)).is_err());
		std::fs::remove_file(root).unwrap();
	}
}
