use litcontainers::*;
use crate::TempoSection;
use std::cmp::Ordering::Equal;
use rayon::prelude::*;

pub fn extract_offset<S>(nc: &S, sr: f64, s: &mut TempoSection, tempo_multiples: &Vec<f32>, doubt_window: f32, doubt_step: f32)
	where S: RowVecStorage<f64>
{
	let start = (s.start() as f64 * sr) as usize;
	let end = ((s.end() as f64 * sr) as usize).min(nc.len());
	let section_length = end - start;

	let min_bpm = s.bpm() - doubt_window / 2.;
	let step_count = (doubt_window / doubt_step) as usize;
	let bpms: Vec<_> = (0..step_count).map(|i| min_bpm + i as f32 * doubt_step).collect();

	let candidates: Vec<_> = bpms.par_iter().cloned().map(|bpm| {
		let samples_per_bar = ((60. / bpm as f64 * sr) * 4.).ceil() as usize;
		let pulse_dim = D!(section_length + samples_per_bar);
		let pulses: Vec<_> = tempo_multiples.iter().cloned().map(|m| {
			litdsp::wave::generate_wave(60. / (bpm * m) as f64, pulse_dim, 0, sr, false)
		}).collect();

		let (mut magnitude, mut offset) = (0., 0.);

		let roi = nc.slice_cols(start..end);
		for i in 0..samples_per_bar {
			let c_magnitude: f64 = pulses.iter().map(|p| {
				roi.as_iter().zip(p.slice_cols(i..i+section_length).as_iter())
					.map(|(a, b)| (a * b).max(0.)).sum::<f64>()
			}).sum();

			if c_magnitude > magnitude {
				magnitude = c_magnitude;
				offset = (start as f64 + -(i as f64)) / sr;
			}
		}

		(magnitude, offset, bpm)
	}).collect();

	let candidate = candidates.into_iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Equal));

	match candidate {
		Some((_, offset, bpm)) => {
			s.set_offset(offset as f32);
			s.set_bpm(bpm);
		},
		None => {}
	}
}

pub fn correct_offset(s: &mut TempoSection, smallest_fraction_shift: i32) {
	let mut offset = s.offset() - s.start();
	let bar_len = 60. / s.bpm() * 4.;
	let fraction_note_len = bar_len / smallest_fraction_shift as  f32;

	if offset < 0. {
		offset += (offset.abs() / bar_len).ceil() * bar_len;
	}
	offset = offset % fraction_note_len;

	s.set_offset(s.start() + offset);
}