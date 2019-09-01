use litcontainers::*;
use litdsp::*;
use num_traits::real::Real;
use std::ops::DivAssign;
use rayon::prelude::*;

#[derive(Debug, Clone, Builder, Getters)]
pub struct NCSettings {
	#[builder(default = "Some(1000.)")]
	log_compression: Option<f64>,
	#[builder(default = "Some(200.)")]
	resample_sr: Option<f64>,
	#[builder(default = "0.3")]
	diff_filter_length: f64,
	#[builder(default = "5.")]
	norm_filter_length: f64,
	#[builder(default = "1.5")]
	smooth_length: f64,
	#[builder(default = "74.")]
	threshold: f64,
	// Db
	#[builder(default = "1000.")]
	resample_precision: f64,
}

pub fn calculate_novelty_curve<S, W, H, B>(s: &S, sr: f64, window_dim: W, hop_dim: H, bands: &ContainerRM<f64, B, U2>, settings: NCSettings)
	-> (RowVec<f64, Dynamic>, f64)
	where S: RowVecStorage<f64>,
	      W: Dim + DimDiv<U2>,
	      <W as DimDiv<U2>>::Output: DimAdd<U1>,
	      H: Dim, B: Dim
{
	let (bands_novelty_curve, stft_sr) = calculate_band_odf(s, sr, window_dim, hop_dim, bands, settings.clone());

	let mut sr = stft_sr;
	let mut novelty_curve = mean_cols(&bands_novelty_curve);

	if let Some(resample_sr) = settings.resample_sr {
		let p = (settings.resample_precision * resample_sr / stft_sr).round() as usize;
		let q = settings.resample_precision as usize;

		novelty_curve = resampling::resample::resample(&novelty_curve, p, q);
		sr = resample_sr; // TODO: its rounded so its not exact.
	}

	let novelty_curve = smooth_filter_subtract(&novelty_curve, stft_sr, settings.smooth_length);

	(novelty_curve, sr)
}

pub fn calculate_band_odf<S, W, H, B>(s: &S, sr: f64, window_dim: W, hop_dim: H, bands: &ContainerRM<f64, B, U2>, settings: NCSettings)
	-> (ContainerRM<f64, B, Dynamic>, f64)
	where S: RowVecStorage<f64>,
	      W: Dim + DimDiv<U2>,
	      <W as DimDiv<U2>>::Output: DimAdd<U1>,
	      H: Dim, B: Dim
{
	let window_length = window_dim.value();

	// Create frequency spectrum.
	let w = window::hanning(window_dim);
	let (stft, stft_sr) = stft::calculate_stft(s, &w, hop_dim, true, sr);
	let thresh = (10.).powf(-settings.threshold / 20.);

	// Normalize it and cut off the noise
	let spe = stft.norm();
	let spe_max = spe.maximum();
	let mut spe = (spe / spe_max).clamp(thresh, 1.);
	if let Some(compression_c) = settings.log_compression {
		spe = (spe * compression_c + 1.).log(1. + compression_c);
	}

	// Make diff filter
	let diff_filter = make_diff_filter(settings.diff_filter_length, stft_sr);
	let diff_len_half = diff_filter.cols() / 2;

	// Make norm filter
	let (norm_filter, norm_filter_sum) = make_norm_filter(settings.norm_filter_length, stft_sr);
	let norm_len_half = norm_filter.cols() / 2;

	// Prepare vals for boundary correction
	let f_half_span = 0..norm_len_half;
	let l_half_span = spe.cols() - norm_len_half..spe.cols();

	let norm_filter_f_slice = norm_filter_sum.slice_cols(f_half_span.clone());
	let norm_filter_f_slice_flipped = norm_filter_f_slice.flip_axis(RowAxis);

	let mut bands_novelty_curve = ContainerRM::zeros(Size::new(bands.row_dim(), spe.col_dim()));

	// TODO: parallelize
	let bins = (bands / (sr / window_length as f64)).round().clamp(0., window_length as f64 / 2.);

	bands_novelty_curve.as_row_slice_iter_mut() // TODO: as_row_slice_par_mut_iter()
		.into_par_iter()
		.zip(bins.as_row_slice_iter().into_par_iter())
		.for_each(|(mut novelty_curve, bin)| {
			let band_data = spe.slice_rows(bin[0] as usize..bin[1] as usize);

			// Calculate band diff
			let band_krn = pad_cols(&band_data, D!(diff_len_half), D!(diff_len_half), true);
			let mut band_diff = conv2_same(&band_krn, &diff_filter).max(0.);
			let mut band_diff = band_diff.slice_cols_mut(diff_len_half - 1..band_diff.cols() - diff_len_half - 1);

			// Normalize band
			let mut norm_curve = conv2_same(&sum_cols(&band_data), &norm_filter);

			// Boundary correction
			norm_curve.slice_cols_mut(f_half_span.clone()).div_assign(&norm_filter_f_slice_flipped);
			norm_curve.slice_cols_mut(l_half_span.clone()).div_assign(&norm_filter_f_slice);

			for mut band_diff_row in band_diff.as_row_slice_iter_mut() {
				band_diff_row /= &norm_curve;
			}

			novelty_curve.copy_from(&sum_cols(&band_diff));
		});

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
	window::hanning(Dynamic::new(diff_len)) * mult_filt
}

fn make_norm_filter(length: f64, sr: f64) -> (RowVec<f64, Dynamic>, RowVec<f64, Dynamic>)
{
	let norm_len = (length * sr).ceil().max(3.) as usize;
	let mut norm_filter = window::hanning(Dynamic::new(norm_len));

	let norm_sum = norm_filter.sum();
	let mut norm_filter_sum = norm_filter.cumsum(RowAxis);
	norm_filter_sum.mapv_inplace(|v| (norm_sum - v) / norm_sum);
	norm_filter /= norm_sum;

	(norm_filter, norm_filter_sum)
}

pub fn smooth_filter_subtract<S>(s: &S, sr: f64, smooth_length: f64)
	-> RowVec<f64, S::Cols>
	where S: RowVecStorage<f64>,
	      S::Cols: DimAdd<Dynamic>, <S::Cols as DimAdd<Dynamic>>::Output: DimSub<U1>
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

pub fn default_audio_bands(sr: f64) -> ContainerRM<f64, U5, U2>
{
	ContainerRM::from_vec(Size::new(U5, U2), &[
		0., 500.,
		500., 1250.,
		1250., 3125.,
		3125., 7812.5,
		7812.5, (sr / 2.).floor()
	])
}