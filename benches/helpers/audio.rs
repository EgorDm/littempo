use std::path::PathBuf;
use litaudio::*;

pub fn setup_audio() -> AudioDeinterleaved<f64, U1, Dynamic> {
	let mut in_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	in_path.push( "assets/test_audio.wav");
	litaudioio::read_audio(in_path.as_path()).unwrap()
}