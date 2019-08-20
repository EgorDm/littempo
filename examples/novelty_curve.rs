use litcontainers::*;
use litaudio::*;
use std::path::{PathBuf};

pub fn setup_audio() -> AudioDeinterleaved<f64, U1, Dynamic> {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("assets/test_audio.wav");
	litaudioio::read_audio(path.as_path()).unwrap()
}

fn main() {
	let audio = setup_audio();
	let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tmp");
	let settings = littempo::TempoExtractionSettingsBuilder::default()
		.save_click_track(true)
		.visualize(true)
		.save_path(path)
		.build().unwrap();

	let tempo_sections = littempo::extract_tempo(&audio, &settings);

	println!("Found tempo sections:");
	for s in &tempo_sections {
		println!("{:#?}", s);
	}
}