use litcontainers::*;
use litaudio::*;
use litplot::plotly::*;
use std::path::{PathBuf, Path};

pub fn setup_audio() -> AudioDeinterleaved<f64, U1, Dynamic> {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push( "assets/test_audio.wav");
	litaudioio::read_audio(path.as_path()).unwrap()
}

fn main() {
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
		Some(1000.),
		None
	);

	let x = RowVec::regspace_rows(U1, novelty_curve.col_dim(), 0.);
	let y = novelty_curve;
	let x_sig = RowVec::regspace_rows(U1, audio.col_dim(), 0.);

	let plot_audio = Plot::new("audio")
		.add_chart(
			LineBuilder::default()
				.identifier("audio")
				.data(XYData::new(
					provider_litcontainer(Fetch::Remote, &x_sig, None).unwrap(),
					provider_litcontainer(Fetch::Remote, &audio, None).unwrap(),
				))
				.name("Audio Wave")
				.build()
				.unwrap()
		);

	let plot = Plot::new("plot_1")
		.add_chart(
			LineBuilder::default()
				.identifier("chart_1")
				.data(XYData::new(
					provider_litcontainer(Fetch::Remote, &x, Some("chart_1_x".into())).unwrap(),
					provider_litcontainer(Fetch::Remote, &y, Some("chart_1_y".into())).unwrap(),
				))
				.name("Novelty Curve")
				.build()
				.unwrap()
		);

	let report = Report::new("Novelty Curve")
		.add_node(plot_audio)
		.add_node(plot);

	let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tmp").join("novelty_curve");
	report.force_save(path.as_path()).unwrap();
}