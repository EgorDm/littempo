use litcontainers::*;
use litaudio::*;
use litplot::plotly::*;
use std::path::{PathBuf, Path};
use itertools::Itertools;
use litdsp::*;

pub fn setup_audio() -> AudioDeinterleaved<f64, U1, Dynamic> {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push( "assets/test_audio.wav");
	litaudioio::read_audio(path.as_path()).unwrap()
}

fn main() {
	let audio = setup_audio();

	let bands = littempo::default_audio_bands(audio.sample_rate() as f64);
	let (novelty_curve, sr) = littempo::calculate_novelty_curve(
		&audio,
		audio.sample_rate() as f64,
		Dynamic::new((1024. * audio.sample_rate() as f64 / 22050.) as usize),
		Dynamic::new((512. * audio.sample_rate() as f64 / 22050.) as usize),
		&bands,
		littempo::NCSettingsBuilder::default().build().unwrap()
	);

	let audio_x = litdsp::wave::calculate_time(audio.col_dim(), audio.sample_rate() as f64);

	// Tempogram
	let tempo_window = (8. * sr) as usize;
	let tempo_hop_size = (sr / 5.).ceil() as usize;
	let bpms = RowVec::regspace_rows(U1, D!(571), 30.);
	let (mut tempogram, tempogram_sr) = littempo::novelty_curve_to_tempogram_dft(
		&novelty_curve,
		sr,
		D!(tempo_window),
		D!(tempo_hop_size),
		&bpms
	);
	normalize_cols_inplace(&mut tempogram, |s| norm_p2_c(s));
	let tempogram_mag = (&tempogram).norm();
	let mut tempogram_mag_t = ContainerRM::zeros(tempogram_mag.row_dim(), tempogram_mag.col_dim());
	tempogram_mag_t.copy_from(&tempogram_mag);

	// Cyclic
	let (cyclic_tempogram, cyclic_tempogram_axis)
		= littempo::tempogram_to_cyclic_tempogram(&tempogram, &bpms, D!(120), 60.);


	let plot = Plot::new("audio")
		.add_chart(
			LineBuilder::default()
				.identifier("audio")
				.data(XYData::new(
					provider_litcontainer(Fetch::Remote, &audio_x, None).unwrap(),
					provider_litcontainer(Fetch::Remote, &audio, None).unwrap(),
				))
				.name("Audio Wave")
				.build()
				.unwrap()
		)
		.add_chart(
			LineBuilder::default()
				.identifier("chart_1")
				.data(XYData::new(
					provider_litcontainer(Fetch::Remote, &litdsp::wave::calculate_time(novelty_curve.col_dim(), sr), Some("chart_1_x".into())).unwrap(),
					provider_litcontainer(Fetch::Remote, &(&novelty_curve / novelty_curve.maximum()), Some("chart_1_y".into())).unwrap(),
				))
				.name("Novelty Curve")
				.build()
				.unwrap()
		);

	let plot2 = Plot::new("tempogram")
		.add_chart(
			HeatmapBuilder::default()
				.data(XYZData::new(
					provider_litcontainer(Fetch::Remote, &litdsp::wave::calculate_time(tempogram.col_dim(), tempogram_sr), None).unwrap(),
					provider_litcontainer(Fetch::Remote, &bpms, None).unwrap(),
					provider_litcontainer(Fetch::Remote, &tempogram_mag_t, None).unwrap(),
				))
				.name("Tempogram")
				.build().unwrap()
		);

	let plot3 = Plot::new("tempogram_cyclic")
		.add_chart(
			HeatmapBuilder::default()
				.data(XYZData::new(
					provider_litcontainer(Fetch::Remote, &litdsp::wave::calculate_time(tempogram.col_dim(), tempogram_sr), None).unwrap(),
					provider_litcontainer(Fetch::Remote, &cyclic_tempogram_axis, None).unwrap(),
					provider_litcontainer(Fetch::Remote, &cyclic_tempogram, None).unwrap(),
				))
				.name("Cyclic Tempogram")
				.build().unwrap()
		);

	let report = Report::new("Novelty Curve")
		.add_node(plot)
		.add_node(plot2)
		.add_node(plot3);

	let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tmp").join("novelty_curve");
	report.force_save(path.as_path()).unwrap();
}