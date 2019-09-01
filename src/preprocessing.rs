use litcontainers::*;
use litdsp::*;
use litdsp::windowed_iter::{WindowedColIter, WindowedIterMut};

pub fn smoothen_tempogram<S, D>(s: &S, window_dim: D)
	-> ContainerRM<f64, S::Rows, S::Cols>
	where S: Storage<f64>, D: Dim
{
	let w_hann = window::hanning(window_dim);
	let w_ones = window::constant(window_dim);
	let w = w_hann + w_ones;

	let ret = apply_window_cols(s, &w);
	let mut ret = subtract_mean(&ret);
	normalize_cols_p1_inplace(&mut ret);
	ret
}

pub fn include_triplets<S, SA>(s: &S, axis: &SA, weight: f64) -> ContainerRM<f64, S::Rows, S::Cols>
	where S: Storage<f64>, SA: RowVecStorage<f64> + StorageSize<Cols=S::Cols>
{
	let triplet_fraction = 3. / 2.;
	let mut ret = ContainerRM::zeros(s.size());
	ret.copy_from(s);

	for (i, mut row) in ret.as_row_slice_iter_mut().enumerate() {
		let triplet_val = (((triplet_fraction * axis[i]) - 1.) % 2.) + 1.;
		let triplet_pos = find_nearest(axis, triplet_val);

		row += &(s.slice_rows(triplet_pos) * row.into_slice() * weight);
	}

	ret
}

pub fn extract_tempo_curve<T, S, TA, SA>(s: &S, axis: &SA) -> RowVec<TA, S::Rows>
	where T: Scalar, S: Storage<T>,
	      TA: Scalar, SA: RowVecStorage<TA> + StorageSize<Cols=S::Cols>
{
	max_bucket_cols(s, axis)
}

pub fn subtract_mean<T, S>(s: &S) -> ContainerRM<T, S::Rows, S::Cols>
	where T: Scalar, S: Storage<T>
{
	let mut ret = ContainerRM::zeros(s.size());
	ret.copy_from(s);
	let mean = s.mean_cols();
	for (mut col, m) in ret.as_col_slice_iter_mut().zip(mean.iter()) {
		col -= m;
	}

	ret
}

pub fn apply_window_cols<T, S, W>(s: &S, w: &RowVec<T, W>) -> ContainerRM<T, S::Rows, S::Cols>
	where T: Scalar, S: Storage<T>, W: Dim
{
	let mut ret = ContainerRM::zeros(s.size());
	let window_dim = w.col_dim();
	let padding = window_dim.value() / 2;
	let mut ci = 0;
	let mut window_iter = WindowedColIter::new_padded(s, window_dim, U1, padding, padding - 1);
	while let Some(mut frame) = window_iter.next_window_mut() {
		for (ri, mut row) in frame.as_row_slice_iter_mut().enumerate() {
			row *= w;
			*ret.get_mut(ri, ci) = row.sum();
		}
		ci += 1;
	}

	ret
}