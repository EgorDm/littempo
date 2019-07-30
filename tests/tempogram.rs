use litcontainers::*;
use litaudio::*;
use std::path::PathBuf;

pub fn setup_audio() -> AudioDeinterleaved<f64, U1, Dynamic> {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push( "assets/test_audio.wav");
	litaudioio::read_audio(path.as_path()).unwrap()
}

#[test]
fn novelty_curve() {
	let audio = setup_audio();

	let bands = ContainerRM::from_vec(U5, U2, &[
		0., 500.,
		500.,    1250.,
		1250.,   3125.,
		3125.,   7812.5,
		7812.5, (audio.sample_rate() as f64 / 2.).floor()
	]);

	let novelty_curve = littempo::calculate_novelty_curve(
		&audio,
		audio.sample_rate() as f64,
		Dynamic::new((1024. * audio.sample_rate() as f64 / 22050.) as usize),
		Dynamic::new((512. * audio.sample_rate() as f64 / 22050.) as usize),
		&bands,
		None,
		None
	);

	let i = 0;
}