use bevy::prelude::*;

/// Occupancy sample used to score hide pockets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HideOccupant {
	pub entity: Entity,
	pub point: Vec3,
}

/// One scored nearby pocket.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HideCandidate {
	pub point: Vec3,
	pub hidden: bool,
	pub occupancy: f32,
	pub away: f32,
}

impl HideCandidate {
	/// Higher is better: concealment, then emptiness, then moving away from the threat.
	pub fn score(self) -> f32 {
		let hide = if self.hidden { 4.0 } else { 0.0 };
		hide + self.away - self.occupancy * 2.0
	}
}

/// Count of other occupants (and hide claims) inside `radius`.
pub fn occupancy_at(
	point: Vec3,
	radius: f32,
	self_entity: Entity,
	occupants: &[HideOccupant],
) -> f32 {
	occupants
		.iter()
		.filter(|occupant| {
			occupant.entity != self_entity
				&& Vec2::new(occupant.point.x, occupant.point.z)
					.distance(Vec2::new(point.x, point.z))
					<= radius
		})
		.count() as f32
}

/// Choose the best nearby pocket. Prefers occlusion, then low occupancy, then away-from-threat.
pub fn pick_hide(
	from: Vec3,
	threat: Vec3,
	samples: &[Vec3],
	self_entity: Entity,
	occupants: &[HideOccupant],
	occupancy_radius: f32,
	hidden: impl Fn(Vec3) -> bool,
) -> Option<Vec3> {
	let current_distance = xz_distance(from, threat);
	samples
		.iter()
		.map(|point| {
			let occupancy = occupancy_at(*point, occupancy_radius, self_entity, occupants);
			let away = (xz_distance(*point, threat) - current_distance).max(0.0).min(8.0) / 8.0;
			HideCandidate { point: *point, hidden: hidden(*point), occupancy, away }
		})
		.max_by(|a, b| a.score().total_cmp(&b.score()))
		.map(|candidate| candidate.point)
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn occupancy_ignores_self() -> anyhow::Result<()> {
		let self_entity = Entity::from_bits(1);
		let other = Entity::from_bits(2);
		let occupants = [
			HideOccupant { entity: self_entity, point: Vec3::ZERO },
			HideOccupant { entity: other, point: Vec3::X },
		];
		assert_eq!(occupancy_at(Vec3::ZERO, 2.0, self_entity, &occupants), 1.0);
		Ok(())
	}

	#[test]
	fn pick_prefers_a_hidden_empty_pocket() -> anyhow::Result<()> {
		let self_entity = Entity::from_bits(1);
		let occupants = [HideOccupant { entity: Entity::from_bits(2), point: Vec3::X * 4.0 }];
		let samples = [Vec3::X * 4.0, Vec3::NEG_X * 4.0];
		let chosen = pick_hide(
			Vec3::ZERO,
			Vec3::X * 20.0,
			&samples,
			self_entity,
			&occupants,
			2.0,
			|point| point.x < 0.0,
		);
		assert_eq!(chosen, Some(Vec3::NEG_X * 4.0));
		Ok(())
	}
}
