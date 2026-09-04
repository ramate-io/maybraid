use bevy::prelude::*;

/// Semantic location represented by a visibility sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpotFeature {
	CenterMass,
	Head,
	LowerBody,
	BodySide,
	HeadSide,
	LowerBodySide,
}

impl SpotFeature {
	pub const fn is_head(self) -> bool {
		matches!(self, Self::Head | Self::HeadSide)
	}
}

/// One world-space visibility sample and its semantic feature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotSample {
	pub point: Vec3,
	pub feature: SpotFeature,
}

/// Approximate bounds used to choose useful line-of-sight samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpotBounds {
	Capsule { radius: f32, half_height: f32 },
}

impl SpotBounds {
	pub fn capsule(radius: f32, half_height: f32) -> Self {
		let radius = radius.max(0.0);
		Self::Capsule { radius, half_height: half_height.max(radius) }
	}

	pub fn center_mass(self, origin: Vec3) -> Vec3 {
		match self {
			Self::Capsule { .. } => origin,
		}
	}

	pub fn head(self, origin: Vec3) -> Vec3 {
		match self {
			Self::Capsule { radius, half_height } => {
				let radius = radius.max(0.0);
				origin + Vec3::Y * (half_height.max(radius) - radius * 0.5).max(0.0)
			}
		}
	}

	/// Character-oriented center, head, lower-body, and side samples.
	///
	/// Samples are ordered so a small budget tests center mass first and head
	/// second. Side offsets are perpendicular to the observer-to-subject line.
	pub fn samples(self, observer: Vec3, origin: Vec3) -> [SpotSample; 9] {
		let Self::Capsule { radius, half_height } = self;
		let radius = radius.max(0.0);
		let half_height = half_height.max(radius);
		let toward =
			Vec3::new(origin.x - observer.x, 0.0, origin.z - observer.z).normalize_or(Vec3::Z);
		let right = Vec3::new(toward.z, 0.0, -toward.x) * (radius * 0.8);
		let center = self.center_mass(origin);
		let head = self.head(origin);
		let lower = origin - Vec3::Y * (half_height * 0.45);
		[
			SpotSample { point: center, feature: SpotFeature::CenterMass },
			SpotSample { point: head, feature: SpotFeature::Head },
			SpotSample { point: lower, feature: SpotFeature::LowerBody },
			SpotSample { point: center + right, feature: SpotFeature::BodySide },
			SpotSample { point: center - right, feature: SpotFeature::BodySide },
			SpotSample { point: head + right, feature: SpotFeature::HeadSide },
			SpotSample { point: head - right, feature: SpotFeature::HeadSide },
			SpotSample { point: lower + right, feature: SpotFeature::LowerBodySide },
			SpotSample { point: lower - right, feature: SpotFeature::LowerBodySide },
		]
	}

	pub const fn sample_count(self) -> usize {
		match self {
			Self::Capsule { .. } => 9,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn capsule_samples_cover_center_head_and_body() -> anyhow::Result<()> {
		let bounds = SpotBounds::capsule(0.4, 0.9);
		let samples = bounds.samples(Vec3::Z * 5.0, Vec3::ZERO);
		assert_eq!(samples[0].point, bounds.center_mass(Vec3::ZERO));
		assert!(samples[1].feature.is_head());
		assert!(samples[1].point.y > samples[0].point.y);
		assert!(samples[3].point.x.abs() > 0.2);
		assert_eq!(samples[3].point.x, -samples[4].point.x);
		Ok(())
	}
}
