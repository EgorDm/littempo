use litcontainers::*;
use crate::Segment;

#[derive(Debug, Clone)]
pub struct TempoSection {
	start: f32,
	end: f32,
	bpm: f32,
	offset: f32
}

impl TempoSection {
	pub fn new(start: f32, end: f32, bpm: f32, offset: f32) -> Self { Self { start, end, bpm, offset } }

	pub fn start(&self) -> f32 { self.start }

	pub fn end(&self) -> f32 { self.end }

	pub fn bpm(&self) -> f32 { self.bpm }

	pub fn set_bpm(&mut self, v: f32) { self.bpm = v }

	pub fn offset(&self) -> f32 { self.offset }

	pub fn set_offset(&mut self, v: f32) { self.offset = v }

	pub fn duration(&self) -> f32 { self.end - self.start }
}

pub fn tempo_segments_to_sections<S>(curve: &S, segments: &Vec<Segment>, sr: f64, ref_tempo: f64)
	-> Vec<TempoSection>
	where S: RowVecStorage<f64>
{
	segments.iter().map(|segment| {
		let start = (*segment.first().unwrap() as f64 / sr) as f32;
		let end = ((segment.last().unwrap() + 1) as f64 / sr) as f32;
		let bpm = (curve[*segment.first().unwrap()] * ref_tempo) as f32;
		TempoSection::new(start, end, bpm, 0.)
	}).collect()
}


pub fn merge_sections(sections: &Vec<TempoSection>, threshold: f32) -> Vec<TempoSection> {
	let mut ret = Vec::new();
	let mut candidates = Vec::new();

	for s in sections {
		if candidates.first().map(|c: &TempoSection|c.bpm > threshold).unwrap_or(false) {
			ret.push(average_sections(&candidates));
			candidates.clear();
		}
		candidates.push(s.clone())
	}

	if !candidates.is_empty() {
		ret.push(average_sections(&candidates));
	}

	ret
}

fn average_sections(sections: &Vec<TempoSection>) -> TempoSection {
	let mut ret = TempoSection::new(
		sections.first().map(|s| s.start).unwrap_or(0.),
		sections.last().map(|s| s.end).unwrap_or(0.),
		0.,
		0.,
	);
	ret.bpm = sections.iter().map(|s| s.bpm * s.duration()).sum::<f32>() / ret.duration();
	ret.offset = sections.iter().map(|s| s.offset * s.duration()).sum::<f32>() / ret.duration();

	ret
}

pub fn split_section(s: TempoSection, sections: &mut Vec<TempoSection>, max_duration: f32) {
	let duration = s.duration();
	if duration < max_duration {
		sections.push(s);
		return;
	}

	let mut s1 = s.clone();
	s1.end = s.start + duration / 2.;
	split_section(s1, sections, max_duration);

	let mut s2 = s.clone();
	s2.start = s.start + duration / 2.;
	split_section(s2, sections, max_duration);
}