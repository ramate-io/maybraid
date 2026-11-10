//! Diagnostic target choice only; never used by renderer admission or selection.
use bevy::prelude::*;

#[derive(Debug)]
pub(super) enum TargetSelector {
	NearestCamera,
	Entity(u64),
	Xz(Vec2),
	Invalid(&'static str),
}
impl TargetSelector {
	pub fn parse(entity: Option<&str>, xz: Option<&str>) -> Self {
		if let Some(entity) = entity {
			return entity.parse().map(Self::Entity).unwrap_or(Self::Invalid("invalid_entity"));
		}
		let Some(xz) = xz else { return Self::NearestCamera };
		let mut parts = xz.split(',');
		let parsed = parts
			.next()
			.and_then(|x| x.trim().parse::<f32>().ok())
			.zip(parts.next().and_then(|z| z.trim().parse::<f32>().ok()));
		match parsed {
			Some((x, z)) if x.is_finite() && z.is_finite() && parts.next().is_none() => {
				Self::Xz(Vec2::new(x, z))
			}
			_ => Self::Invalid("invalid_xz"),
		}
	}
	pub fn name(&self) -> &'static str {
		match self {
			Self::NearestCamera => "nearest_camera",
			Self::Entity(_) => "entity",
			Self::Xz(_) => "containing_xz",
			Self::Invalid(_) => "invalid",
		}
	}
	pub fn anchor(&self) -> Option<Vec2> {
		if let Self::Xz(xz) = self {
			Some(*xz)
		} else {
			None
		}
	}
	pub fn error(&self) -> Option<&'static str> {
		if let Self::Invalid(error) = self {
			Some(error)
		} else {
			None
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TargetCandidate {
	pub entity: u64,
	pub family: &'static str,
	pub center: Vec3,
	pub min: Vec3,
	pub max: Vec3,
}
impl TargetCandidate {
	pub fn contains_xz(&self, anchor: Vec2) -> bool {
		anchor.x >= self.min.x
			&& anchor.x <= self.max.x
			&& anchor.y >= self.min.z
			&& anchor.y <= self.max.z
	}
	pub fn compare(&self, other: &Self, anchor: Vec2) -> std::cmp::Ordering {
		let distance = |candidate: &Self| {
			Vec2::new(candidate.center.x, candidate.center.z).distance_squared(anchor)
		};
		distance(self)
			.total_cmp(&distance(other))
			.then(self.family.cmp(other.family))
			.then_with(|| {
				let a = [self.min.x, self.min.y, self.min.z, self.max.x, self.max.y, self.max.z];
				let b =
					[other.min.x, other.min.y, other.min.z, other.max.x, other.max.y, other.max.z];
				a.into_iter()
					.zip(b)
					.map(|(a, b)| a.total_cmp(&b))
					.find(|order| !order.is_eq())
					.unwrap_or(std::cmp::Ordering::Equal)
			})
			.then(self.entity.cmp(&other.entity))
	}
	pub fn choose(best: Option<Self>, candidate: Self, anchor: Vec2) -> Option<Self> {
		if !candidate.contains_xz(anchor) {
			return best;
		}
		Some(best.filter(|old| !candidate.compare(old, anchor).is_lt()).unwrap_or(candidate))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn finite_selector_parse_and_entity_precedence() {
		assert!(matches!(TargetSelector::parse(None, None), TargetSelector::NearestCamera));
		assert!(
			matches!(TargetSelector::parse(None,Some(" 150, -350 ")),TargetSelector::Xz(v) if v == Vec2::new(150.,-350.))
		);
		for text in ["NaN,0", "0,inf", "1", "1,2,3", "1,", "", "1e99,0"] {
			assert!(
				matches!(
					TargetSelector::parse(None, Some(text)),
					TargetSelector::Invalid("invalid_xz")
				),
				"{text}"
			);
		}
		assert!(matches!(
			TargetSelector::parse(Some("42"), Some("NaN,0")),
			TargetSelector::Entity(42)
		));
		assert!(matches!(
			TargetSelector::parse(Some("bad"), Some("0,0")),
			TargetSelector::Invalid("invalid_entity")
		));
	}
	fn candidate(entity: u64, family: &'static str, center: Vec3) -> TargetCandidate {
		TargetCandidate { entity, family, center, min: Vec3::splat(-2.), max: Vec3::splat(2.) }
	}
	#[test]
	fn containing_bounds_wait_and_deterministic_nearest_ties() {
		let anchor = Vec2::ZERO;
		let far = TargetCandidate {
			min: Vec3::splat(10.),
			max: Vec3::splat(20.),
			..candidate(1, "A", Vec3::ZERO)
		};
		assert!(TargetCandidate::choose(None, far, anchor).is_none());
		let candidates = [
			far,
			candidate(9, "B", Vec3::X),
			candidate(8, "A", Vec3::X),
			candidate(7, "Z", Vec3::new(0.5, 99., 0.)),
		];
		let select = |items: Vec<TargetCandidate>| {
			items
				.into_iter()
				.fold(None, |best, c| TargetCandidate::choose(best, c, anchor))
				.unwrap()
				.entity
		};
		assert_eq!(select(candidates.to_vec()), 7);
		assert_eq!(select(candidates.into_iter().rev().collect()), 7);
		let a = candidate(9, "A", Vec3::X);
		let b = candidate(8, "B", Vec3::X);
		assert_eq!(TargetCandidate::choose(Some(b), a, anchor).unwrap().entity, 9);
		let changed = TargetCandidate { min: Vec3::new(-1., -2., -2.), ..a };
		assert!(a.compare(&changed, anchor).is_lt());
		assert!(a.contains_xz(Vec2::new(2., -2.)));
	}
}
