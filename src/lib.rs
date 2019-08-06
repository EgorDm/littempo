#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate derive_getters;

pub mod novelty_curve;
pub mod tempogram;
pub mod preprocessing;
pub mod tempo_curve;

pub use novelty_curve::*;
pub use tempogram::*;
pub use preprocessing::*;
pub use tempo_curve::*;

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		assert_eq!(2 + 2, 4);
	}
}
