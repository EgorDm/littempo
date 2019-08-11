use litcontainers::*;
use litaudio::*;
use std::f64;
use crate::TempoSection;
use std::path::Path;
use litaudioio::error::Error;

pub fn save_tempo_click_track<C, L, P, S>(path: &Path, a: S, sections: &Vec<TempoSection>, click_fraction: u32) -> Result<(), Error>
	where C: Dim, L: Dim, P: SamplePackingType, S: AudioStorageMut<f32, C, L, P>
{
	let click_track = click_track(sections, a.sample_rate() as f64, click_fraction);
	let mut output = a;

	match click_track {
		Some(click_track) => {
			for mut ch in output.as_row_slice_mut_iter() {
				ch.as_iter_mut().zip(click_track.as_iter()).for_each(|(o, c)| *o = clamp(*o + *c, -1., 1.))
			}
		},
		None => {}
	}

	litaudioio::write_audio(path, &output)
}


pub fn click_track(sections: &Vec<TempoSection>, sr: f64, click_fraction: u32) -> Option<AudioDeinterleaved<f32, U1, Dynamic>> {
	if sections.is_empty() { return None; }
	let len = ((sections.last().unwrap().end() - sections.first().unwrap().start()) as f64 * sr).round() as usize;
	let mut ret = AudioDeinterleaved::new(DeinterleavedStorage::zeros(U1,  D!(len)), sr as i32);

	for s in sections {
		let clicks = click_track_from_section(s, sr, click_fraction);
		let start = (s.start() as f64 * sr).round() as usize;
		let end = clicks.sample_count().min(ret.sample_count());
		ret.slice_samples_mut(start..end).copy_from(&clicks.slice_samples(0..(end - start)))
	}

	Some(ret)
}

pub fn click_sound_custom(sr: f64, duration: f32, freq: f64) -> RowVec<f32, Dynamic> {
	let angular_freq = 2. * f64::consts::PI * freq / sr as f64;
	let len = (duration as f64 * sr) as usize;
	let mut click = RowVec::linspace_rows(U1, D!(len), 0., -10.).exp2();
	click *= &(RowVec::regspace_rows(U1, D!(len), 0.) * angular_freq as f32).sin();
	click
}

pub fn click_sound(sr: f64) -> RowVec<f32, Dynamic> { click_sound_custom(sr, 0.1, 1000.) }

pub fn click_track_from_section(s: &TempoSection, sr: f64, note_fraction: u32) -> AudioDeinterleaved<f32, U1, Dynamic> {
	click_track_from_tempo(s.bpm(), s.offset(), D!((s.duration() as f64 * sr).round() as usize), sr, note_fraction)
}

pub fn click_track_from_tempo<D: Dim>(bpm: f32, offset: f32, length: D, sr: f64, note_fraction: u32) -> AudioDeinterleaved<f32, U1, D> {
	let end = (length.value() as f64 / sr) as f32;
	let bar_len = 60. / bpm * 4.;
	let frac_note_len = bar_len / note_fraction as f32;
	let mut positions = Vec::new();
	let mut position = offset;

	while position < end {
		positions.push(position);
		position += frac_note_len;
	}

	click_track_from_positions(&positions, sr, length)
}

pub fn click_track_from_positions<D: Dim>(p: &Vec<f32>, sr: f64, length: D) -> AudioDeinterleaved<f32, U1, D>
{
	let mut ret = AudioDeinterleaved::new(DeinterleavedStorage::zeros(U1, length), sr.round() as i32);
	let click = click_sound(sr);

	for pos in p {
		if *pos < 0. { continue; }
		let pos = (*pos as f64 * sr).round() as usize;
		if pos >= length.value() { continue; }

		let click_len = click.col_count().min(ret.col_count() - pos);
		ret.slice_samples_mut(pos..pos + click_len).copy_from(&click.slice_cols(0..click_len));
	}

	ret
}