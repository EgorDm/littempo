use litcontainers::*;
use litdsp::*;

pub fn novelty_curve_to_tempogram_dft<C, S, W, H, F>(s: &S, sr: f64, window_dim: W, hop_dim: H, bpms: &RowVec<f64, F>)
	-> (ContainerRM<c64, F, Dynamic>, f64)
	where C: Dim + DimAdd<Dynamic>,
	      S: Storage<f64, U1, C>,
	      W: Dim + DimDiv<U2>,
	      <W as DimDiv<U2>>::Output: DimAdd<U1>,
	      <C as DimAdd<Dynamic>>::Output: DimAdd<Dynamic>,
	      H: Dim, F: Dim
{
	let w = window::hanning(window_dim);
	let window_length = window_dim.value();
	let window_length_half = (window_length as f32 / 2.).round() as usize;

	let padded_s = pad_cols(s, D!(window_length_half), D!(window_length_half), false);

	// Compute tempogram
	let (mut tg, sr) = stft::calculate_fourier_coefficients(&padded_s, &w, hop_dim, &(bpms / 60.), sr);

	// Normalize
	tg /= (window_length as f64).sqrt() * w.sum() / window_length as f64;

	(tg, sr)
}

pub fn tempogram_to_cyclic_tempogram<C, F, O>(tg: &ContainerRM<c64, F, C>, bpms: &RowVec<f64, F>, octave_divider: O, ref_tempo: f64)
	-> (ContainerRM<f64, O, C>, RowVec<f64, O>)
	where F: Dim, C: Dim, O: Dim
{
	let min_bpm = bpms.minimum();
	let max_bpm = bpms.maximum();
	let ref_octave = ref_tempo / min_bpm;
	let min_octave = (min_bpm / ref_tempo).log2().round();
	let max_octave = (max_bpm / ref_tempo).log2().round() + 1.;

	let mag_tempogram = tg.norm();

	let log_bpm_count = ((max_octave - 1. / octave_divider.value() as f64) - min_octave) / (1. / octave_divider.value() as f64);
	let log_bpm = RowVec::regspace_step_rows(
		U1,
		D!(log_bpm_count.floor() as usize),
		min_octave as f64,
		1. / octave_divider.value() as f64
	).exp2() * ref_tempo;
	let mut log_tempogram = ContainerRM::zeros(log_bpm.col_dim(), mag_tempogram.col_dim());
	interp1_nearest_cols(&bpms.t(), &mag_tempogram, &log_bpm.t(), &mut log_tempogram);

	let mut cyclic_tempogram = ContainerRM::zeros(octave_divider, mag_tempogram.col_dim());
	let end_pos = log_bpm.as_iter().cloned().enumerate().filter(|(_, v)| *v < max_bpm).last().unwrap().0;
	for (i, mut row) in cyclic_tempogram.as_row_slice_mut_iter().enumerate() {
		let range = (i..end_pos).step_by(octave_divider.value());
		let range_size = range.len();
		for j in range {
			row += &log_tempogram.slice_rows(j);
		}
		row /= range_size as f64;
	}

	let y_axis = log_bpm.slice_cols(SizedRange::new(0, octave_divider)) * (ref_octave / ref_tempo);
	(cyclic_tempogram, y_axis)
}

pub fn bpm_to_cyclic(bpm: f64, ref_tempo: f64) -> f64 {
	let mins = (bpm / 60.).log2().floor();
	let diff = (ref_tempo * 2f64.powf(mins + 1.)) -  (ref_tempo * 2f64.powf(mins));
	bpm - (60. * 2f64.powf(mins)) / diff + 1.
}