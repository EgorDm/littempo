use std::path::{PathBuf};
use litcontainers::*;
use litaudio::{AudioDeinterleaved, AudioStorage};
use littempo::{NCSettingsBuilder};


#[test]
fn novelty_curve() {
	let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let target: RowVec<f64, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_novelty_curve.lit")).unwrap();
	let audio: AudioDeinterleaved<f64, U1, Dynamic> = litaudioio::read_audio(&crate_path.join("assets/test_audio.wav")).unwrap();

	let bands = ContainerRM::from_vec(U5, U2, &[
		0., 500.,
		500.,    1250.,
		1250.,   3125.,
		3125.,   7812.5,
		7812.5, (audio.sample_rate() as f64 / 2.).floor()
	]);


	let (novelty_curve, _) = littempo::calculate_novelty_curve(
		&audio,
		audio.sample_rate() as f64,
		Dynamic::new((1024. * audio.sample_rate() as f64 / 22050.) as usize),
		Dynamic::new((512. * audio.sample_rate() as f64 / 22050.) as usize),
		&bands,
		NCSettingsBuilder::default().build().unwrap()
	);

	for (target, result) in target.iter().zip(novelty_curve.iter()) {
		assert!((target - result).abs() < 0.000001);
	}
}