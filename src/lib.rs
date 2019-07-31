#[macro_use]
extern crate derive_builder;
#[macro_use]
extern crate derive_getters;

pub mod novelty_curve;
pub mod tempogram;

pub use novelty_curve::*;
pub use tempogram::*;

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		assert_eq!(2 + 2, 4);
	}
}
