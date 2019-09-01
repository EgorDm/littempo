use litcontainers::*;
use litaudio::*;
use litdsp::*;
use litplot::plotly::*;
use std::path::PathBuf;
use std::cmp::Ordering::Equal;
use crate::TempoSection;

#[builder(pattern = "owned")]
#[derive(Debug, Builder, Getters)]
pub struct TempoExtractionSettings {
	/// Analysis band bins.
	#[builder(default = "None")]
	analysis_band_bins: Option<ContainerRM<f64, Dynamic, U2>>,
	/// Analysis window length in seconds used for calculating tempogram.
	#[builder(default = "8.")]
	tempo_window: f64,
	/// Analysis window hop length in seconds used for calculating tempogram.
	#[builder(default = "0.2")]
	tempo_hop_size: f64,
	/// BPMs which to check for tempo peaks.
	#[builder(default = "RowVec::regspace(Size::new(U1, D!(571)), RowAxis, 30.)")]
	scan_bpms: RowVec<f64, Dynamic>,
	/// Reference tempo defining the partition of BPM into tempo octaves for calculating cyclic tempogram.
	#[builder(default = "60.")]
	ref_tempo: f64,
	/// Determines the amount of bins and octave (based on ref_tempo) is divided into thus the dimensionality of cyclic tempogram.
	#[builder(default = "120")]
	octave_divider: usize,
	/// Weight of the triplet intensity which will be adeed to its base intensity.
	#[builder(default = "3.")]
	triplet_weight: f64,
	/// Length in seconds over which the tempogram will be stabilized to extract a steady tempo.
	#[builder(default = "20.")]
	smooth_length: f64,
	/// Minimum length for a tempo section in seconds.
	#[builder(default = "10.")]
	min_section_length: f32,
	/// Maximum section length in seconds after which section is split in half.
	#[builder(default = "40.")]
	max_section_length: f32,
	/// Tempo multiples to consider when searching for correct offset.
	#[builder(default = "vec![1., 2., 4., 6.]")]
	tempo_multiples: Vec<f32>,
	/// BPM around which the real bpm will be chosen.
	#[builder(default = "120.")]
	preferred_bpm: f32,
	/// Precision of the BPM before correction.
	#[builder(default = "0.5")]
	bpm_rounding_precision: f32,
	/// Window around candidate bpm which to search for a more fine and correct bpm.
	#[builder(default = "2.")]
	bpm_doubt_window: f32,
	/// Steps size to take winthin doubt window to finetune bpm.
	#[builder(default = "0.1")]
	bpm_doubt_step: f32,
	/// Threshold to merge similar bpm together.
	#[builder(default = "0.5")]
	bpm_merge_threshold: f32,
	/// Allow correction to shift the offset by given note subdivision.
	#[builder(default = "4")]
	smallest_fraction_shift: i32,
	/// Verbose.
	#[builder(default = "false")]
	verbose: bool,
	/// Visualize.
	#[builder(default = "false")]
	visualize: bool,
	/// Save the click track.
	#[builder(default = "false")]
	save_click_track: bool,
	/// Click every xth note.
	#[builder(default = "12")]
	click_fraction: u32,
	/// Path to save the plot and clicktrack in if enabled.
	#[builder(default = "std::env::current_dir().unwrap()")]
	save_path: PathBuf,
}

impl TempoExtractionSettings {
	pub fn get_tempo_window(&self, sr: f64) -> usize {
		(self.tempo_window as f64 * sr).round() as usize
	}

	pub fn get_tempo_hop_size(&self, sr: f64) -> usize {
		(self.tempo_hop_size as f64 * sr).round() as usize
	}

	pub fn get_smooth_length(&self, sr: f64) -> usize {
		(self.smooth_length as f64 * sr).round() as usize
	}

	pub fn get_min_section_length(&self, sr: f64) -> usize {
		(self.min_section_length as f64 * sr).round() as usize
	}
}

pub fn extract_tempo<P, S>(a: &S, settings: &TempoExtractionSettings) -> Vec<TempoSection>
	where P: SamplePackingType, S: AudioStorage<f64, P> + StorageSize<Rows=U1>
{
	let sr = a.sample_rate() as f64;

	if *settings.verbose() { println!("Processing audio file.") }

	if *settings.verbose() { println!(" - Calculating novelty curve") }
	// Calculate novelty curve / odf
	let bands = settings.analysis_band_bins().as_ref().map(|c| c.clone_owned()).unwrap_or({
		let ret = crate::default_audio_bands(sr);
		ret.transmute_dims(
			Size::new(D!(ret.rows()), ret.col_dim()),
			ret.strides()
		).owned()
	});
	let (novelty_curve, nc_sr) = crate::calculate_novelty_curve(
		a,
		sr,
		Dynamic::new((1024. * sr / 22050.) as usize),
		Dynamic::new((512. * sr as f64 / 22050.) as usize),
		&bands,
		crate::NCSettingsBuilder::default().build().unwrap()
	);

	if *settings.verbose() { println!(" - Calculating tempogram") }
	// Make Tempogram
	let (mut tempogram, tempogram_sr) = crate::novelty_curve_to_tempogram_dft(
		&novelty_curve,
		nc_sr,
		D!(settings.get_tempo_window(nc_sr)),
		D!(settings.get_tempo_hop_size(nc_sr)),
		settings.scan_bpms()
	);
	// Normalize tempogram
	normalize_cols_inplace(&mut tempogram, |s| norm_p2_c(s));
	let tempogram_mag = (&tempogram).norm();
	let mut tempogram_mag_t = ContainerRM::zeros(Size::new(tempogram_mag.row_dim(), tempogram_mag.col_dim()));
	tempogram_mag_t.copy_from(&tempogram_mag);

	if *settings.verbose() { println!(" - Calculating cyclic tempogram") }
	// Make Cyclic Tempogram
	let (cyclic_tempogram, cyclic_tempogram_axis)
		= crate::tempogram_to_cyclic_tempogram(&tempogram, settings.scan_bpms(), D!(*settings.octave_divider()), *settings.ref_tempo());

	if *settings.verbose() { println!(" - Preprocessing and cleaning tempogram") }
	// Preprocess tempogram
	let triplet_corrected_cyclic_tempogram = crate::include_triplets(&cyclic_tempogram, &cyclic_tempogram_axis, *settings.triplet_weight());
	let mut smooth_tempogram = crate::smoothen_tempogram(
		&triplet_corrected_cyclic_tempogram,
		D!(settings.get_smooth_length(tempogram_sr))
	);
	smooth_tempogram.as_iter_mut().for_each(|v| if *v < 0. { *v = 0.; } else {});

	if *settings.verbose() { println!(" - Tempo peaks extraction") }
	// Tempo curve extraction
	let tempo_curve = crate::extract_tempo_curve(&smooth_tempogram, &cyclic_tempogram_axis);
	let tempo_curve = crate::correct_curve_by_length(&tempo_curve, settings.get_min_section_length(tempogram_sr));

	let tempo_segments = crate::split_curve(&tempo_curve);
	let tempo_sections = crate::tempo_segments_to_sections(&tempo_curve, &tempo_segments, tempogram_sr, *settings.ref_tempo());
	let tempo_sections_tmp = crate::merge_sections(&tempo_sections, *settings.bpm_merge_threshold());

	let mut tempo_sections = Vec::new();
	for s in tempo_sections_tmp {
		crate::split_section(s, &mut tempo_sections, *settings.max_section_length());
	}

	if *settings.verbose() { println!(" - Tempo offset estimation") }
	// Correct bpm height
	for s in tempo_sections.iter_mut() {
		let best_multiple = settings.tempo_multiples().iter().cloned()
			.max_by(|a, b| (settings.preferred_bpm() - a * s.bpm()).partial_cmp(&(settings.preferred_bpm() - b * s.bpm())).unwrap_or(Equal));
		s.set_bpm(best_multiple.unwrap_or(1.) * s.bpm());
		s.set_bpm((s.bpm() / settings.bpm_rounding_precision()).round() * settings.bpm_rounding_precision());

		// Correct offset
		crate::extract_offset(&novelty_curve, nc_sr, s, settings.tempo_multiples(), *settings.bpm_doubt_window(), *settings.bpm_doubt_step());
		crate::correct_offset(s, *settings.smallest_fraction_shift());
	}

	if *settings.verbose() { println!(" - Done!") }

	// Save a click track
	if *settings.save_click_track() {
		let path = settings.save_path().join("click_track.mp3");
		let mut click_audio = DeinterleavedStorage::zeros(Size::new(a.channel_dim(), a.sample_dim())).into_audio(a.sample_rate(), Deinterleaved);
		click_audio.as_iter_mut().zip(a.as_iter()).for_each(|(o, i)| *o = *i as f32);
		crate::save_tempo_click_track(&path, click_audio, &tempo_sections, *settings.click_fraction()).unwrap();
	}

	// Plot data
	if *settings.visualize() {
		let audio_x = litdsp::wave::calculate_time(a.col_dim(), sr);
		let plot = Plot::new("audio")
			.add_chart(
				LineBuilder::default()
					.identifier("audio")
					.data(XYData::new(
						provider_litcontainer(Fetch::Remote, &audio_x, None).unwrap(),
						provider_litcontainer(Fetch::Remote, a, None).unwrap(),
					))
					.name("Audio Wave")
					.build()
					.unwrap()
			)
			.add_chart(
				LineBuilder::default()
					.identifier("chart_1")
					.data(XYData::new(
						provider_litcontainer(Fetch::Remote, &litdsp::wave::calculate_time(novelty_curve.col_dim(), nc_sr), Some("chart_1_x".into())).unwrap(),
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
						provider_litcontainer(Fetch::Remote, settings.scan_bpms(), None).unwrap(),
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

		let plot4 = Plot::new("smooth_tempogram")
			.add_chart(
				HeatmapBuilder::default()
					.data(XYZData::new(
						provider_litcontainer(Fetch::Remote, &litdsp::wave::calculate_time(tempogram.col_dim(), tempogram_sr), None).unwrap(),
						provider_litcontainer(Fetch::Remote, &cyclic_tempogram_axis, None).unwrap(),
						provider_litcontainer(Fetch::Remote, &smooth_tempogram, None).unwrap(),
					))
					.name("Smooth Tempogram")
					.build().unwrap()
			)
			.add_chart(
				LineBuilder::default()
					.data(XYData::new(
						provider_litcontainer(Fetch::Remote, &litdsp::wave::calculate_time(tempogram.col_dim(), tempogram_sr), None).unwrap(),
						provider_litcontainer(Fetch::Remote, &tempo_curve, None).unwrap()
					))
					.name("Tempo Curve")
					.build().unwrap()
			);

		let report = Report::new("Novelty Curve")
			.add_node(plot)
			.add_node(plot2)
			.add_node(plot3)
			.add_node(plot4);

		let path = settings.save_path().join("plot");
		report.force_save(path.as_path()).unwrap();
	}

	tempo_sections
}