//! Duration print helpers matching the POI scheme-timing tables.

use std::time::Duration;

/// A timed sample formatted the way existing benches print it.
#[derive(Clone, Copy, Debug)]
pub struct PublishedDuration(pub Duration);

impl PublishedDuration {
	pub fn micros_2(self) -> String {
		format!("{:.2}", self.0.as_secs_f64() * 1_000_000.0)
	}

	pub fn compact(self) -> String {
		let ms = self.0.as_secs_f64() * 1_000.0;
		if ms >= 1_000.0 {
			format!("{:.2} s", self.0.as_secs_f64())
		} else {
			format!("{:.2} ms", ms)
		}
	}

	pub fn millis_f64(self) -> f64 {
		self.0.as_secs_f64() * 1_000.0
	}
}
