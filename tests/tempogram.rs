use std::path::{PathBuf};
use litcontainers::*;
use litaudio::{AudioDeinterleaved, AudioStorage, AudioDeinterleavedC};
use littempo::{NCSettingsBuilder};


#[test]
fn novelty_curve() {
	let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let target: RowVec<f64, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_novelty_curve.lit")).unwrap();
	let audio: AudioDeinterleavedC<f64, U1, Dynamic> = litaudioio::read_audio(&crate_path.join("assets/test_audio.wav")).unwrap();

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

#[test]
fn tempogram() {
	let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let novelty_curve: RowVec<f64, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_novelty_curve.lit")).unwrap();
	let target: ContainerRM<c64, Dynamic, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_tempogram.lit")).unwrap();

	let nc_sr: f64 = 200.;
	let bpms = RowVec::regspace(Size::new(U1, D!(571)), RowAxis, 30.);
	let tempo_window = (8. * nc_sr) as usize;
	let tempo_hop_size = (nc_sr / 5.).ceil() as usize;

	let (mut tempogram, _tempogram_sr) = littempo::novelty_curve_to_tempogram_dft(
		&novelty_curve,
		nc_sr,
		D!(tempo_window),
		D!(tempo_hop_size),
		&bpms
	);

	litdsp::normalize_cols_inplace(&mut tempogram, |s| norm_p2_c(s));
	let tempogram_mag = tempogram.norm();
	let target_mag = target.norm();

	for (target, result) in target_mag.iter().zip(tempogram_mag.iter()) {
		assert!((target - result).abs() < 0.000001);
	}
}

#[test]
fn cyclic_tempogram() {
	let crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let tempogram: ContainerRM<c64, Dynamic, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_tempogram.lit")).unwrap();
	let target: ContainerRM<f64, Dynamic, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_cyclic_tempogram.lit")).unwrap();
	let target_axis: RowVec<f64, Dynamic> = litio::read_binary_file(&crate_path.join("assets/test_audio_cyclic_tempogram_axis.lit")).unwrap();

	let bpms = RowVec::regspace(Size::new(U1, D!(571)), RowAxis, 30.);
	let (cyclic_tempogram, cyclic_tempogram_axis)
		= littempo::tempogram_to_cyclic_tempogram(&tempogram, &bpms, D!(120), 60.);

	for (target, result) in target.iter().zip(cyclic_tempogram.iter()) {
		assert!((target - result).abs() < 0.000001);
	}

	for (target, result) in target_axis.iter().zip(cyclic_tempogram_axis.iter()) {
		assert!((target - result).abs() < 0.000001);
	}
}