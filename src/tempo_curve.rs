use litcontainers::*;
use num_traits::Signed;

type Segment = Vec<usize>;

pub fn correct_curve_by_length<T, R, S>(curve: &S, min_length: usize) -> RowVec<T, R>
	where T: ElementaryScalar + Signed, R: Dim, S: RowVecStorage<T, R>
{
	// Split measurements in segments with same value
	let segments = split_curve(curve);

	// Take small segments and join if needed
	let small_segments: Vec<_> = segments.iter()
		.filter(|s| s.len() < min_length).cloned().collect();

	// Delete the small segments by replaing their value to neareast outside their boundaries
	let mut ret = rvec_zeros![curve.col_dim()];
	ret.copy_from(curve);
	for segment in small_segments {
		let (start, end) = (*segment.first().unwrap(), *segment.last().unwrap());
		let before = if start > 0 { curve[start] } else { T::max_val() };
		let after = if end + 1 < curve.row_count() { curve[end + 1] } else { T::max_val() };
		let target = if (ret[start] - before).abs() > (ret[start] - after).abs() { after } else { before };
		if target == T::max_val() { continue; }

		for i in segment { ret[i] = target };
	}

	ret
}

pub fn correct_curve_by_confidence() {
	unimplemented!() // TODO: assume normal distribution. Keep exceptional changes
}

fn split_curve<T, R, S>(curve: &S) -> Vec<Segment>
	where T: ElementaryScalar, R: Dim, S: RowVecStorage<T, R>
{
	let mut ret = Vec::new();
	let mut current_section = Vec::new();
	let mut current_value = T::max_val();

	for (i, v) in curve.as_iter().enumerate() {
		if current_value == *v {
			current_section.push(i);
		} else {
			if !current_section.is_empty() {
				ret.push(current_section);
			}
			current_section = vec![i];
			current_value = *v;
		}
	}

	if !current_section.is_empty() {
		ret.push(current_section);
	}

	ret
}

fn join_adjacent_segments(segments: Vec<Segment>) -> Vec<Segment> {
	let mut ret = Vec::new();
	for segment in segments {
		if ret.last().and_then(|s: &Segment| s.last())
			.and_then(|li| segment.first().map(|fi| *li == fi - 1)) == Some(true) {
			ret.last_mut().unwrap().extend(segment);
		} else {
			ret.push(segment);
		}
	}
	ret
}

/*
fn merge_segements(segments: Vec<Segment>, ) {

}*/
