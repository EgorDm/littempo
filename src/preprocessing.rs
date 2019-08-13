use litcontainers::*;
use litdsp::*;
use litdsp::windowed_iter::{WindowedColIter, WindowedIterMut};

pub fn smoothen_tempogram<R, C, S, D>(s: &S, window_dim: D)
	-> ContainerRM<f64, R, C>
	where R: Dim, C: Dim, S: Storage<f64, R, C>, D: Dim
{
	let w_hann = window::hanning(window_dim);
	let w_ones = window::constant(window_dim);
	let w = w_hann + &w_ones;

	let ret = apply_window_cols(s, &w);
	let mut ret = subtract_mean(&ret);
	normalize_cols_p1_inplace(&mut ret);
	ret
}

pub fn include_triplets<R, C, S, SA>(s: &S, axis: &SA, weight: f64) -> ContainerRM<f64, R, C>
	where R: Dim, C: Dim, S: Storage<f64, R, C>, SA: RowVecStorage<f64, C>
{
	let triplet_fraction = 3. / 2.;
	let mut ret = ContainerRM::zeros(s.row_dim(), s.col_dim());
	ret.copy_from(s);

	for (i, mut row) in ret.as_row_slice_mut_iter().enumerate() {
		let triplet_val = (((triplet_fraction * axis[i]) - 1.) % 2.) + 1.;
		let triplet_pos = find_nearest(axis, triplet_val);

		row += &(s.slice_rows(triplet_pos) * &row * weight);
	}

	ret
}

pub fn extract_tempo_curve<T, R, C, S, TA, SA>(s: &S, axis: &SA) -> RowVec<TA, R>
	where T: ElementaryScalar, R: Dim, C: Dim, S: Storage<T, R, C>,
	      TA: ElementaryScalar, SA: RowVecStorage<TA, C>
{
	max_bucket_cols(s, axis)
}

pub fn subtract_mean<T, R, C, S>(s: &S) -> ContainerRM<T, R, C>
	where T: ElementaryScalar, R: Dim, C: Dim, S: Storage<T, R, C>
{
	let mut ret = ContainerRM::zeros(s.row_dim(), s.col_dim(), );
	ret.copy_from(s);
	let mean = mean_cols(s);
	for (mut col, m) in ret.as_col_slice_mut_iter().zip(mean.iter()) {
		col -= m;
	}

	ret
}

pub fn apply_window_cols<T, R, C, S, W>(s: &S, w: &RowVec<T, W>) -> ContainerRM<T, R, C>
	where T: ElementaryScalar, R: Dim, C: Dim, S: Storage<T, R, C>, W: Dim
{
	let mut ret = ContainerRM::zeros(s.row_dim(), s.col_dim());
	let window_dim = w.col_dim();
	let padding = window_dim.value() / 2;
	let mut ci = 0;
	let mut window_iter = WindowedColIter::new_padded(s, window_dim, U1, padding, padding - 1);
	while let Some(mut frame) = window_iter.next_window_mut() {
		for (ri, mut row) in frame.as_row_slice_mut_iter().enumerate() {
			row *= w;
			*ret.get_mut(ri, ci) = row.sum();
		}
		ci += 1;
	}

	ret
}