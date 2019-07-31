use std::path::{PathBuf};
use litcontainers::*;
use litaudio::{AudioDeinterleaved, AudioStorage};
use littempo::{NCSettingsBuilder};


#[test]
fn novelty_curve() {
	let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let target: RowVec<f64, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_novelty_curve.lit")).unwrap();
	let audio: AudioDeinterleaved<f64, U1, Dynamic> = litaudioio::read_audio(&crate_path.join("assets/test_audio.wav")).unwrap();

	let bands = littempo::default_audio_bands(audio.sample_rate() as f64);

	let (novelty_curve, _) = littempo::calculate_novelty_curve(
		&audio,
		audio.sample_rate() as f64,
		Dynamic::new((1024. * audio.sample_rate() as f64 / 22050.) as usize), // TODO: move default into settings
		Dynamic::new((512. * audio.sample_rate() as f64 / 22050.) as usize),
		&bands,
		NCSettingsBuilder::default().build().unwrap()
	);

	for (target, result) in target.iter().zip(novelty_curve.iter()) {
		assert!((target - result).abs() < 0.000001);
	}
}