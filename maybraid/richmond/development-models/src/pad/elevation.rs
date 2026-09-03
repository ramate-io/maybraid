//! Local height field on a pad footprint. Flatten now; grading later.

/// How a pad sets height inside its footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadElevation {
	/// Replace incoming elevation with a constant height (building terrace).
	Flatten { height: f32 },
}

impl PadElevation {
	pub fn height(self) -> f32 {
		match self {
			Self::Flatten { height } => height,
		}
	}
}
