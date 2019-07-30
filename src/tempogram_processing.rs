use litcontainers::*;
use litdsp::*;
use litdsp::functions::*;
use litdsp::stft::calculate_stft;
use num_traits::real::Real;
use std::ops::DivAssign;

// TODO: check whether rowslice can be correctly converted to colslice. Lossless

pub fn calculate_band_odf<C, S, W, H, B>(s: &S, sr: f64, window_dim: W, hop_dim: H, bands: &ContainerRM<f64, B, U2>, log_compression: Option<f64>)
	-> (ContainerRM<f64, B, Dynamic>, f64)
	where C: Dim, S: Storage<f64, U1, C>,
	      W: Dim + DimDiv<U2>,
	      <W as DimDiv<U2>>::Output: DimAdd<U1>,
	      H: Dim, B: Dim
{
	let window_length = window_dim.value();
	let hop_length = hop_dim.value();

	// Create frequency spectrum.
	let w = window::hanning(window_dim);
	let (stft, stft_sr) = calculate_stft(s, &w, hop_dim, true, sr);
	let thresh = (10.).powf(-74. / 20.); // -74 db TODO: settings

	// Normalize it and cut off the noise
	let spe = stft.norm();
	let spe_max = spe.maximum();
	let mut spe = (spe / spe_max).clamp(thresh, 1.);
	if let Some(compression_c) = log_compression {
		spe = (spe * compression_c + 1.).log(1. + compression_c);
	}

	// Make diff filter
	let diff_filter = make_diff_filter(0.3, stft_sr);
	let diff_len_half = diff_filter.col_count() / 2;

	// Make norm filter
	let (norm_filter, norm_filter_sum) = make_norm_filter(5., stft_sr);
	let norm_len_half = norm_filter.col_count() / 2;

	// Prepare vals for boundary correction
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

	(bands_novelty_curve, stft_sr)
}

fn make_diff_filter(length: f64, sr: f64) -> RowVec<f64, Dynamic>
{
	// Diff length
	let diff_len = (length * sr).ceil().max(5.);
	let diff_len = (2. * (diff_len / 2.).round() + 1.) as usize;
	let diff_len_half = diff_len / 2;

	// Make diff filter. TODO: can better
	let left = rvec_value![Dynamic::new(diff_len_half); 1.];
	let mid = rvec_zeros![U1; f64];
	let right = rvec_value![Dynamic::new(diff_len_half); -1.];
	let mult_filt = join_cols!(left, mid, right);
	window::hanning(Dynamic::new(diff_len)) * &mult_filt
}

fn make_norm_filter(length: f64, sr: f64) -> (RowVec<f64, Dynamic>, RowVec<f64, Dynamic>)
{
	let norm_len = (length * sr).ceil().max(3.) as usize;
	let norm_len_half = norm_len / 2;
	let mut norm_filter = window::hanning(Dynamic::new(norm_len));

	let norm_sum = norm_filter.sum();
	let mut norm_filter_sum = cumsum_rows(&norm_filter);
	norm_filter_sum.mapv_inplace(|v| (norm_sum - v) / norm_sum);
	norm_filter /= norm_sum;

	(norm_filter, norm_filter_sum)
}

pub fn calculate_novelty_curve<C, S, W, H, B>(s: &S, sr: f64, window_dim: W, hop_dim: H, bands: &ContainerRM<f64, B, U2>, log_compression: Option<f64>, resample_sr: Option<f64>)
	-> (RowVec<f64, Dynamic>, f64)
	where C: Dim, S: Storage<f64, U1, C>,
	      W: Dim + DimDiv<U2>,
	      <W as DimDiv<U2>>::Output: DimAdd<U1>,
	      H: Dim,
	      B: Dim
{
	let (bands_novelty_curve, stft_sr) = calculate_band_odf(s, sr, window_dim, hop_dim, bands, log_compression);

	let mut sr = stft_sr;
	let mut novelty_curve = mean_cols(&bands_novelty_curve);

	if let Some(resample_sr) = resample_sr {
		let p = (1000. * resample_sr / stft_sr).round() as usize;
		let q = 1000;

		novelty_curve = resampling::resample::resample(&novelty_curve, p, q);
		sr = resample_sr; // TODO: its rounded so its not exact.
	}

	// TODO: resample and smooth filter subtract
	let novelty_curve = smooth_filter_subtract(&novelty_curve, stft_sr, 1.5);

	(novelty_curve, sr)
}

pub fn smooth_filter_subtract<C, S>(s: &S, sr: f64, smooth_length: f64)
	-> RowVec<f64, C>
	where C: Dim, S: RowVecStorage<f64, C>,
			C: DimAdd<Dynamic>, <C as DimAdd<Dynamic>>::Output: DimSub<U1>
{
	let smooth_length = (smooth_length * sr).ceil().max(3.) as usize;
	let mut smooth_filter = window::hanning(D!(smooth_length));
	smooth_filter /= smooth_filter.sum();
	let mut local_avg = conv2_same(s, &smooth_filter);

	for (local_avg, amplitude) in local_avg.as_iter_mut().zip(s.as_iter()) {
		*local_avg = (*amplitude - *local_avg).max(0.);
	}

	local_avg
}