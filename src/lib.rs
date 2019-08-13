#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate derive_getters;

pub mod novelty_curve;
pub mod tempogram;
pub mod preprocessing;
pub mod tempo_curve;
pub mod tempo_sections;
pub mod offset_extraction;
pub mod click_track;
pub mod tempo_extraction;

pub use novelty_curve::*;
pub use tempogram::*;
pub use preprocessing::*;
pub use tempo_curve::*;
pub use tempo_sections::*;
pub use offset_extraction::*;
pub use click_track::*;
pub use tempo_extraction::*;

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		assert_eq!(2 + 2, 4);
	}
}
