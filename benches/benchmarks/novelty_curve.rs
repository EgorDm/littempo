use criterion::Criterion;
use crate::helpers::setup_audio;
use litaudio::AudioStorage;
use litcontainers::*;
use litdsp::stft;

fn calculate_novelty_curve_benchmark(c: &mut Criterion) {
	let audio = setup_audio();

	let bands = ContainerRM::from_vec(U5, U2, &[
		0., 500.,
		500.,    1250.,
		1250.,   3125.,
		3125.,   7812.5,
		7812.5, (audio.sample_rate() as f64 / 2.).floor()
	]);

	c.bench_function("calculate_novelty_curve", move |b| b.iter(|| {
		let novelty_curve = littempo::calculate_novelty_curve(
			&audio,
			audio.sample_rate() as f64,
			Dynamic::new((1024. * audio.sample_rate() as f64 / 22050.) as usize),
			Dynamic::new((512. * audio.sample_rate() as f64 / 22050.) as usize),
			&bands,
			None,
			None
		);
	}));
}

criterion_group!{
    name = benchmark;
    config = Criterion::default().sample_size(10);
    targets = calculate_novelty_curve_benchmark
}