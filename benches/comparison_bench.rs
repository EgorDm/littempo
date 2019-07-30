#[macro_use]
extern crate criterion;

mod helpers;
mod benchmarks;

use benchmarks::*;

criterion_main!(
	novelty_curve::benchmark,
);