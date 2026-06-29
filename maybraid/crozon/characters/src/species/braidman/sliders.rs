//! Braidman rig slider values.
//!
//! Values are multipliers on top of the species baseline. `1.0` means "leave
//! Braidman's baseline intact", not "write an identity transform over the bind
//! pose".

/// Minimal body slider set for the first concepts pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BraidmanSliders {
	pub shoulder_width: f32,
	pub hip_width: f32,
	pub chest_thickness: f32,
}

impl Default for BraidmanSliders {
	fn default() -> Self {
		Self { shoulder_width: 1.0, hip_width: 1.0, chest_thickness: 1.0 }
	}
}

impl BraidmanSliders {
	pub fn new(shoulder_width: f32, hip_width: f32, chest_thickness: f32) -> Self {
		Self { shoulder_width, hip_width, chest_thickness }.clamped()
	}

	pub fn with_shoulder_width(mut self, value: f32) -> Self {
		self.shoulder_width = value;
		self.clamped()
	}

	pub fn with_hip_width(mut self, value: f32) -> Self {
		self.hip_width = value;
		self.clamped()
	}

	pub fn with_chest_thickness(mut self, value: f32) -> Self {
		self.chest_thickness = value;
		self.clamped()
	}

	pub fn clamped(mut self) -> Self {
		self.shoulder_width = self.shoulder_width.clamp(0.8, 1.2);
		self.hip_width = self.hip_width.clamp(0.8, 1.4);
		self.chest_thickness = self.chest_thickness.clamp(0.8, 1.2);
		self
	}

	pub fn status_label(self) -> String {
		format!(
			"shoulder_width={:.2} hip_width={:.2} chest_thickness={:.2}",
			self.shoulder_width, self.hip_width, self.chest_thickness,
		)
	}
}
