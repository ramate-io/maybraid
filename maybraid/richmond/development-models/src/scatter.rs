//! Deterministic, recipe-driven jittered-grid scattering.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use procedural_common::{Bounds2, SeededHash};

use crate::cell::{sample_confines_yaw, yawed_plan_aabb_extent};

#[derive(Debug, Clone)]
pub struct ScatterChoice<K> {
	pub kind: K,
	pub weight: f32,
	pub min_footprint: f32,
	pub max_footprint: f32,
}

#[derive(Debug, Clone)]
pub struct ScatterRecipe<K> {
	pub grid_side: usize,
	pub min_count: usize,
	pub max_count: usize,
	pub cell_inset: f32,
	pub jitter: f32,
	pub clearance: f32,
	pub choices: Vec<ScatterChoice<K>>,
}

#[derive(Debug, Clone)]
pub struct ScatterCandidate<K> {
	pub slot: usize,
	pub center: Vec2,
	pub yaw: f32,
	pub footprint: Vec2,
	pub kind: K,
}

#[derive(Debug, Clone)]
pub struct ScatterPlan<K> {
	pub target_count: usize,
	pub candidates: Vec<ScatterCandidate<K>>,
}

impl<K: Clone> ScatterRecipe<K> {
	pub fn plan(&self, cell: Aabb3d, root: SeededHash) -> ScatterPlan<K> {
		assert!(self.grid_side > 1, "scatter grid requires at least two rows");
		assert!(!self.choices.is_empty(), "scatter recipe requires a choice");

		let count_range = self.max_count.saturating_sub(self.min_count) + 1;
		let target_count = self.min_count + (root.unit(101) * count_range as f32).floor() as usize;
		let mut slots: Vec<(usize, f32)> = (0..self.grid_side * self.grid_side)
			.map(|slot| (slot, root.unit(200 + slot as u32)))
			.collect();
		slots.sort_by(|a, b| a.1.total_cmp(&b.1));

		let candidates = slots
			.into_iter()
			.map(|(slot, _)| {
				let hash = SeededHash::new(
					root.seed.wrapping_add((slot as u32 + 1).wrapping_mul(0x9E37_79B9)),
				);
				let choice = self.pick_choice(hash.unit(1));
				ScatterCandidate {
					slot,
					center: self.candidate_center(cell, slot, hash),
					yaw: sample_confines_yaw(hash.unit(4)),
					footprint: Vec2::new(
						lerp(choice.min_footprint, choice.max_footprint, hash.unit(2)),
						lerp(choice.min_footprint, choice.max_footprint, hash.unit(3)),
					),
					kind: choice.kind.clone(),
				}
			})
			.collect();
		ScatterPlan { target_count, candidates }
	}

	pub fn collision_bounds(&self, candidate: &ScatterCandidate<K>) -> Bounds2 {
		let half =
			yawed_plan_aabb_extent(candidate.footprint.x, candidate.footprint.y, candidate.yaw)
				* 0.5 + Vec2::splat(self.clearance);
		Bounds2 { min: candidate.center - half, max: candidate.center + half }
	}

	fn candidate_center(&self, cell: Aabb3d, slot: usize, hash: SeededHash) -> Vec2 {
		let ix = slot % self.grid_side;
		let iz = slot / self.grid_side;
		let min = Vec2::new(cell.min.x + self.cell_inset, cell.min.z + self.cell_inset);
		let max = Vec2::new(cell.max.x - self.cell_inset, cell.max.z - self.cell_inset);
		let denominator = (self.grid_side - 1) as f32;
		let base = Vec2::new(
			lerp(min.x, max.x, ix as f32 / denominator),
			lerp(min.y, max.y, iz as f32 / denominator),
		);
		let jitter = Vec2::new(
			lerp(-self.jitter, self.jitter, hash.unit(5)),
			lerp(-self.jitter, self.jitter, hash.unit(6)),
		);
		(base + jitter).clamp(min, max)
	}

	fn pick_choice(&self, unit: f32) -> &ScatterChoice<K> {
		let total: f32 = self.choices.iter().map(|choice| choice.weight.max(0.0)).sum();
		assert!(total > f32::EPSILON, "scatter recipe requires a positive choice weight");
		let mut selection = unit * total;
		for choice in &self.choices {
			let weight = choice.weight.max(0.0);
			if weight > 0.0 && selection < weight {
				return choice;
			}
			selection -= weight;
		}
		self.choices
			.iter()
			.rev()
			.find(|choice| choice.weight > 0.0)
			.expect("a choice has positive weight")
	}
}

pub fn bounds_intersect(a: Bounds2, b: Bounds2) -> bool {
	a.min.x <= b.max.x && b.min.x <= a.max.x && a.min.y <= b.max.y && b.min.y <= a.max.y
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::Vec3;

	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	enum Kind {
		Disabled,
		Enabled,
	}

	#[test]
	fn plan_respects_zero_weight_choices_and_count_range() {
		let recipe = ScatterRecipe {
			grid_side: 4,
			min_count: 3,
			max_count: 5,
			cell_inset: 20.0,
			jitter: 4.0,
			clearance: 2.0,
			choices: vec![
				ScatterChoice {
					kind: Kind::Disabled,
					weight: 0.0,
					min_footprint: 1.0,
					max_footprint: 1.0,
				},
				ScatterChoice {
					kind: Kind::Enabled,
					weight: 1.0,
					min_footprint: 4.0,
					max_footprint: 8.0,
				},
			],
		};
		let cell = Aabb3d::new(Vec3::splat(100.0), Vec3::new(100.0, 1.0, 100.0));
		let plan = recipe.plan(cell, SeededHash::new(42));

		assert!((3..=5).contains(&plan.target_count));
		assert_eq!(plan.candidates.len(), 16);
		assert!(plan.candidates.iter().all(|candidate| candidate.kind == Kind::Enabled));
	}
}
