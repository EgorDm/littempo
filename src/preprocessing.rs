use litcontainers::*;

pub fn smoothen_tempogram() {
	unimplemented!()
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

