//! Explicit, bounded microphone recording for native voice messages.
//! Audio is held in memory only and is cleared when canceled or after upload.
use cpal::Sample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
	sync::{
		Arc, Mutex,
		atomic::{AtomicBool, Ordering},
		mpsc,
	},
	time::{Duration, Instant},
};

const RATE: u32 = 48_000;
pub(crate) const MAX_SECONDS: u64 = 120;
const MAX_SAMPLES: usize = RATE as usize * MAX_SECONDS as usize;
const MAX_OGG_BYTES: usize = 2 * 1024 * 1024;
const INVALID: &str = "Could not encode this recording as a voice message";

pub(crate) struct Message {
	pub(crate) ogg: Vec<u8>,
	pub(crate) duration_secs: f64,
	pub(crate) waveform: String,
}

enum Control {
	Stop,
	Cancel,
}

pub(crate) enum Event {
	Recording(Instant),
	Encoding,
	Ready(Result<Message, &'static str>),
}

/// Owns microphone startup, capture and encoding on one worker thread. The UI only sends bounded
/// controls and polls bounded status/result events; no audio device work runs during a frame.
pub(crate) struct Capture {
	control: mpsc::SyncSender<Control>,
	events: mpsc::Receiver<Event>,
	started: Option<Instant>,
}

impl Capture {
	pub(crate) fn start(
		device_id: Option<String>,
		context: egui::Context,
	) -> Result<Self, &'static str> {
		let (control, controls) = mpsc::sync_channel(1);
		let (events, receive) = mpsc::sync_channel(2);
		std::thread::Builder::new()
			.name("evita-voice-message-capture".into())
			.spawn(move || capture_worker(device_id.as_deref(), controls, events, context))
			.map_err(|_| "Could not start voice-message capture")?;
		Ok(Self {
			control,
			events: receive,
			started: None,
		})
	}

	pub(crate) fn stop(&self) {
		let _ = self.control.try_send(Control::Stop);
	}

	pub(crate) fn cancel(&self) {
		let _ = self.control.try_send(Control::Cancel);
	}

	pub(crate) fn duration(&self) -> Duration {
		self.started
			.map_or(Duration::ZERO, |started| started.elapsed())
			.min(Duration::from_secs(MAX_SECONDS))
	}

	pub(crate) fn try_event(&mut self) -> Result<Option<Event>, ()> {
		match self.events.try_recv() {
			Ok(event) => match event {
				Event::Recording(started) => {
					self.started = Some(started);
					Ok(Some(Event::Recording(started)))
				}
				other => Ok(Some(other)),
			},
			Err(mpsc::TryRecvError::Empty) => Ok(None),
			Err(mpsc::TryRecvError::Disconnected) => Err(()),
		}
	}
}

fn capture_worker(
	device_id: Option<&str>,
	controls: mpsc::Receiver<Control>,
	events: mpsc::SyncSender<Event>,
	context: egui::Context,
) {
	let recorder = match Recorder::start(device_id) {
		Ok(recorder) => recorder,
		Err(error) => {
			let _ = events.send(Event::Ready(Err(error)));
			context.request_repaint();
			return;
		}
	};
	let started = Instant::now();
	if events.send(Event::Recording(started)).is_err() {
		return;
	}
	context.request_repaint();
	loop {
		match controls.recv_timeout(Duration::from_millis(100)) {
			Ok(Control::Cancel) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
			Ok(Control::Stop) => break,
			Err(mpsc::RecvTimeoutError::Timeout)
				if recorder.limit_reached.load(Ordering::Acquire) =>
			{
				break;
			}
			Err(mpsc::RecvTimeoutError::Timeout) if started.elapsed().as_secs() < MAX_SECONDS => {
				context.request_repaint();
			}
			Err(mpsc::RecvTimeoutError::Timeout) => break,
		}
	}
	if events.send(Event::Encoding).is_err() {
		return;
	}
	context.request_repaint();
	let result = recorder.finish();
	let _ = events.send(Event::Ready(result));
	context.request_repaint();
}

pub(crate) struct Recorder {
	stream: Option<cpal::Stream>,
	samples: Arc<Mutex<Vec<f32>>>,
	failed: Arc<AtomicBool>,
	limit_reached: Arc<AtomicBool>,
}

impl Recorder {
	pub(crate) fn start(device_id: Option<&str>) -> Result<Self, &'static str> {
		discord_voice::audio::authorize_microphone_for_recording()?;
		let host = cpal::default_host();
		let device = if let Some(id) = device_id {
			let id = id
				.parse::<cpal::DeviceId>()
				.map_err(|_| "The selected microphone is unavailable")?;
			host.device_by_id(&id)
				.ok_or("The selected microphone is unavailable")?
		} else {
			host.default_input_device()
				.ok_or("No microphone is available")?
		};
		let supported = input_config(&device)?;
		let channels = usize::from(supported.channels());
		let sample_rate = supported.sample_rate();
		let sample_format = supported.sample_format();
		let config = supported.config();
		let samples = Arc::new(Mutex::new(Vec::with_capacity(MAX_SAMPLES)));
		let failed = Arc::new(AtomicBool::new(false));
		let limit_reached = Arc::new(AtomicBool::new(false));
		let stream = match sample_format {
			cpal::SampleFormat::F32 => input_stream::<f32>(
				&device,
				config,
				samples.clone(),
				failed.clone(),
				limit_reached.clone(),
				channels,
				sample_rate,
			)?,
			cpal::SampleFormat::I16 => input_stream::<i16>(
				&device,
				config,
				samples.clone(),
				failed.clone(),
				limit_reached.clone(),
				channels,
				sample_rate,
			)?,
			cpal::SampleFormat::I32 => input_stream::<i32>(
				&device,
				config,
				samples.clone(),
				failed.clone(),
				limit_reached.clone(),
				channels,
				sample_rate,
			)?,
			cpal::SampleFormat::U16 => input_stream::<u16>(
				&device,
				config,
				samples.clone(),
				failed.clone(),
				limit_reached.clone(),
				channels,
				sample_rate,
			)?,
			_ => return Err("The microphone uses an unsupported audio format"),
		};
		stream.play().map_err(
			|_| "Could not start the microphone; check system permission and device selection",
		)?;
		Ok(Self {
			stream: Some(stream),
			samples,
			failed,
			limit_reached,
		})
	}

	/// Stop capture immediately, then encode on the capture worker thread.
	fn finish(mut self) -> Result<Message, &'static str> {
		drop(self.stream.take());
		if self.failed.load(Ordering::Acquire) {
			return Err("Microphone capture stopped unexpectedly; try again");
		}
		let samples = {
			let mut buffer = self
				.samples
				.lock()
				.map_err(|_| "Could not finish the microphone recording")?;
			std::mem::take(&mut *buffer)
		};
		if samples.len() < RATE as usize / 5 || samples.len() > MAX_SAMPLES {
			return Err("Record at least 0.2 seconds and no more than two minutes");
		}
		encode(&samples)
	}
}

fn input_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, &'static str> {
	let supported: Vec<_> = device
		.supported_input_configs()
		.map_err(|_| "Microphone formats are unavailable")?
		.take(64)
		.collect();
	if let Some(config) = supported
		.iter()
		.filter(|config| (1..=8).contains(&config.channels()))
		.filter(|config| supported_format(config.sample_format()))
		.filter_map(|config| config.clone().try_with_sample_rate(RATE))
		.min_by_key(|config| config.channels())
	{
		return Ok(config);
	}
	let config = device
		.default_input_config()
		.map_err(|_| "Default microphone format is unavailable")?;
	if !(1..=8).contains(&config.channels())
		|| !(8_000..=192_000).contains(&config.sample_rate())
		|| !supported_format(config.sample_format())
	{
		return Err("Microphone format is unsupported; choose another device");
	}
	Ok(config)
}

fn supported_format(format: cpal::SampleFormat) -> bool {
	matches!(
		format,
		cpal::SampleFormat::F32
			| cpal::SampleFormat::I16
			| cpal::SampleFormat::I32
			| cpal::SampleFormat::U16
	)
}

fn input_stream<T>(
	device: &cpal::Device,
	config: cpal::StreamConfig,
	samples: Arc<Mutex<Vec<f32>>>,
	failed: Arc<AtomicBool>,
	limit_reached: Arc<AtomicBool>,
	channels: usize,
	sample_rate: u32,
) -> Result<cpal::Stream, &'static str>
where
	T: cpal::SizedSample,
	f32: cpal::FromSample<T>,
{
	let mut previous = None::<f32>;
	let mut phase = 0.0_f64;
	let step = f64::from(sample_rate) / f64::from(RATE);
	let callback_failed = failed.clone();
	let callback_limit_reached = limit_reached.clone();
	device
		.build_input_stream(
			config,
			move |data: &[T], _| {
				let Ok(mut output) = samples.try_lock() else {
					callback_failed.store(true, Ordering::Release);
					return;
				};
				for frame in data.chunks_exact(channels) {
					let sample = frame
						.iter()
						.map(|value| value.to_sample::<f32>())
						.sum::<f32>() / channels as f32;
					if let Some(prior) = previous {
						while phase < 1.0 {
							if output.len() == MAX_SAMPLES {
								callback_limit_reached.store(true, Ordering::Release);
								return;
							}
							output.push((prior + (sample - prior) * phase as f32).clamp(-1.0, 1.0));
							phase += step;
						}
						phase -= 1.0;
					}
					previous = Some(sample);
				}
			},
			move |_| failed.store(true, Ordering::Release),
			None,
		)
		.map_err(|_| "Could not open the microphone; check system permission and device selection")
}

fn encode(samples: &[f32]) -> Result<Message, &'static str> {
	use opus2::{Application, Bitrate, Channels, Encoder, Signal};
	let duration_secs = samples.len() as f64 / f64::from(RATE);
	if !duration_secs.is_finite() || !(0.2..=MAX_SECONDS as f64).contains(&duration_secs) {
		return Err(INVALID);
	}
	let mut encoder = Encoder::new(RATE, Channels::Mono, Application::Voip).map_err(|_| INVALID)?;
	encoder
		.set_bitrate(Bitrate::Bits(32_000))
		.map_err(|_| INVALID)?;
	encoder.set_signal(Signal::Voice).map_err(|_| INVALID)?;
	let mut ogg = Vec::with_capacity(samples.len() / 4);
	let mut sequence = 0_u32;
	write_page(
		&mut ogg,
		&mut sequence,
		2,
		0,
		b"OpusHead\x01\x01\0\0\x80\xbb\0\0\0\0",
		true,
	)?;
	let vendor = b"Evita";
	let mut tags = b"OpusTags".to_vec();
	tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
	tags.extend_from_slice(vendor);
	tags.extend_from_slice(&0u32.to_le_bytes());
	write_page(&mut ogg, &mut sequence, 0, 0, &tags, false)?;
	let mut packet = [0_u8; 1275];
	let mut padded = [0_f32; 960];
	let mut granule = 312_u64;
	for (index, chunk) in samples.chunks(960).enumerate() {
		padded.fill(0.0);
		padded[..chunk.len()].copy_from_slice(chunk);
		let len = encoder
			.encode_float(&padded, &mut packet)
			.map_err(|_| INVALID)?;
		granule = granule.saturating_add(chunk.len() as u64);
		let final_page = (index + 1) * 960 >= samples.len();
		write_page(
			&mut ogg,
			&mut sequence,
			if final_page { 4 } else { 0 },
			granule,
			&packet[..len],
			false,
		)?;
		if ogg.len() > MAX_OGG_BYTES {
			return Err(INVALID);
		}
	}
	Ok(Message {
		ogg,
		duration_secs,
		waveform: waveform(samples),
	})
}

fn write_page(
	output: &mut Vec<u8>,
	sequence: &mut u32,
	flags: u8,
	granule: u64,
	packet: &[u8],
	bos: bool,
) -> Result<(), &'static str> {
	if packet.is_empty() || packet.len() > 65_025 {
		return Err(INVALID);
	}
	let mut lacing = vec![255_u8; packet.len() / 255];
	let remainder = packet.len() % 255;
	// A zero lace terminates packets whose lengths are an exact multiple of 255.
	lacing.push(remainder as u8);
	if lacing.len() > 255 {
		return Err(INVALID);
	}
	let mut header = Vec::with_capacity(27 + lacing.len());
	header.extend_from_slice(b"OggS");
	header.push(0);
	header.push(flags | if bos { 2 } else { 0 });
	header.extend_from_slice(&granule.to_le_bytes());
	header.extend_from_slice(&0x4556_4954_u32.to_le_bytes());
	header.extend_from_slice(&sequence.to_le_bytes());
	header.extend_from_slice(&0_u32.to_le_bytes());
	header.push(lacing.len() as u8);
	header.extend_from_slice(&lacing);
	let checksum = ogg_crc(&header, packet);
	header[22..26].copy_from_slice(&checksum.to_le_bytes());
	output.extend_from_slice(&header);
	output.extend_from_slice(packet);
	*sequence = sequence.checked_add(1).ok_or(INVALID)?;
	Ok(())
}

fn ogg_crc(header: &[u8], packet: &[u8]) -> u32 {
	let mut crc = 0_u32;
	for byte in header.iter().chain(packet) {
		crc ^= u32::from(*byte) << 24;
		for _ in 0..8 {
			crc = if crc & 0x8000_0000 != 0 {
				(crc << 1) ^ 0x04c1_1db7
			} else {
				crc << 1
			};
		}
	}
	crc
}

fn waveform(samples: &[f32]) -> String {
	const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
	let block = samples.len().div_ceil(256).max(1);
	let bytes: Vec<u8> = samples
		.chunks(block)
		.take(256)
		.map(|chunk| {
			let peak = chunk
				.iter()
				.filter(|sample| sample.is_finite())
				.map(|sample| sample.abs())
				.fold(0.0_f32, f32::max);
			(peak * 255.0).round() as u8
		})
		.collect();
	let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
	for chunk in bytes.chunks(3) {
		let a = chunk[0];
		let b = *chunk.get(1).unwrap_or(&0);
		let c = *chunk.get(2).unwrap_or(&0);
		encoded.push(ALPHABET[(a >> 2) as usize] as char);
		encoded.push(ALPHABET[(((a & 3) << 4) | (b >> 4)) as usize] as char);
		encoded.push(if chunk.len() > 1 {
			ALPHABET[(((b & 15) << 2) | (c >> 6)) as usize] as char
		} else {
			'='
		});
		encoded.push(if chunk.len() > 2 {
			ALPHABET[(c & 63) as usize] as char
		} else {
			'='
		});
	}
	encoded
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn voice_encoder_produces_bounded_ogg_opus_with_waveform_metadata() {
		let samples: Vec<_> = (0..RATE / 2)
			.map(|index| ((index as f32 * std::f32::consts::TAU * 440.0 / RATE as f32).sin()) * 0.2)
			.collect();
		let message = encode(&samples).unwrap();
		assert_eq!(&message.ogg[..4], b"OggS");
		assert!(message.ogg.windows(8).any(|window| window == b"OpusHead"));
		assert!(message.ogg.windows(8).any(|window| window == b"OpusTags"));
		assert!((message.duration_secs - 0.5).abs() < f64::EPSILON);
		assert!(!message.waveform.is_empty() && message.waveform.len() <= 344);
		assert!(message.ogg.len() < MAX_OGG_BYTES);
	}

	#[test]
	fn encoder_rejects_too_short_or_too_long_recordings() {
		assert!(encode(&[0.0; 9_599]).is_err());
		assert!(encode(&vec![0.0; MAX_SAMPLES + 1]).is_err());
	}
}
