use litcontainers::*;
use litdsp::*;
use litdsp::functions::*;
use litdsp::stft::calculate_stft;
use num_traits::real::Real;
use std::ops::DivAssign;

// TODO: check whether rowslice can be correctly converted to colslice. Lossless

pub fn calculate_novelty_curve<C, S, W, H, B>(s: &S, sr: f64, window_dim: W, hop_dim: H, bands: &ContainerRM<f64, B, U2>, log_compression: Option<f64>, resample_sr: Option<usize>)
	-> RowVec<f64, Dynamic>
	where C: Dim, S: Storage<f64, U1, C>,
	      W: Dim + DimDiv<U2>,
	      <W as DimDiv<U2>>::Output: DimAdd<U1>,
	      H: Dim,
	      B: Dim
{
	let window_length = window_dim.value();
	let hop_length = hop_dim.value();

	// TODO: use settings builder
	// Create frequency spectrum. Normalize it. Cut off the noise
	let w = window::hanning(window_dim);

	let thresh = (10.).powf(-74. / 20.); // -74 db
	let (stft, stft_sr) = calculate_stft(s, &w, hop_dim, true, sr);
	let spe = stft.norm();
	let spe_max = spe.maximum();
	let mut spe = (spe / spe_max).clamp(thresh, 1.);
	if let Some(compression_c) = log_compression {
		spe = (spe * compression_c + 1.).log(1. + compression_c);
	}

	// Diff length
	let diff_len = 0.3; // TODO move into settings
	let diff_len = (diff_len * sr / hop_length as f64).ceil().max(5.);
	let diff_len = (2. * (diff_len / 2.).round() + 1.) as usize;
	let diff_len_half = diff_len / 2;

	// Make diff filter
	let left = rvec_value![Dynamic::new(diff_len_half); 1.];
	let mid = rvec_zeros![U1; f64];
	let right = rvec_value![Dynamic::new(diff_len_half); -1.];
	let mult_filt = join_cols!(left, mid, right);
	let diff_filter = window::hanning(Dynamic::new(diff_len)) * &mult_filt;

	// Make norm filter
	let norm_len = 5.;// TODO move into settings
	let norm_len = (norm_len * sr / hop_length as f64).ceil().max(3.) as usize;
	let norm_len_half = norm_len / 2;
	let mut norm_filter = window::hanning(Dynamic::new(norm_len));
	let norm_sum = norm_filter.sum();
	let mut norm_filter_sum = cumsum_rows(&norm_filter);
	norm_filter_sum.mapv_inplace(|v| (norm_sum - v) / norm_sum);
	norm_filter /= norm_sum;
	let f_half_span = 0..norm_len_half;
	let l_half_span = spe.col_count() - norm_len_half..spe.col_count();

	let norm_filter_f_slice = norm_filter_sum.slice_cols(f_half_span.clone());
	let norm_filter_f_slice_flipped = norm_filter_f_slice.flip_rows();

	let mut bands_novelty_curve = ContainerRM::zeros(bands.row_dim(), spe.col_dim());

	let bins = (bands / (sr / window_length as f64)).round().clamp(0., window_length as f64 / 2.);
	for (bin, mut novelty_curve) in bins.as_row_slice_iter().zip(bands_novelty_curve.as_row_slice_mut_iter()) {
		let band_data = spe.slice_rows(bin[0] as usize..bin[1] as usize);

		// Calculate band diff
		let band_krn = pad_cols(&band_data, D!(diff_len_half), D!(diff_len_half), true);
		let mut band_diff = conv2_same(&band_krn, &diff_filter).max(0.);
		let mut band_diff = band_diff.slice_cols_mut(diff_len_half - 1..band_diff.col_count() - diff_len_half - 1);

		// Normalize band
		let mut norm_curve = conv2_same(&sum_cols(&band_data), &norm_filter);
		// Boundary correction
		norm_curve.slice_cols_mut(f_half_span.clone()).div_assign(&norm_filter_f_slice_flipped);
		norm_curve.slice_cols_mut(l_half_span.clone()).div_assign(&norm_filter_f_slice);

		for mut band_diff_row in band_diff.as_row_slice_mut_iter() {
			band_diff_row /= &norm_curve;
		}

		novelty_curve.copy_from(&sum_cols(&band_diff));
	}

	let novelty_curve = mean_cols(&bands_novelty_curve);

	// TODO: resample and smooth filter subtract

	novelty_curve
}