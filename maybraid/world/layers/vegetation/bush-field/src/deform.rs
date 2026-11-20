//! CPU evaluation of deformed branch/leaf attachments. Matches the wood/foliage shaders.

use bevy::math::{Vec2, Vec3};
use vegetation_shaders::wesl::{
	attachment_frame, branch_rest_sample, deform_along_curve, leaf_surface_vertex, CurveSample,
	LocalFrame,
};

use crate::layout::{BranchDescriptor, LeafDescriptor, PlantDescriptor};

/// Bounded parent walk (max depth 2 + trunk). No recursive shader traversal.
pub const MAX_ATTACH_HOPS: u32 = 3;

impl BranchDescriptor {
	/// Rest sample in the branch's authored space (world for a trunk, parent-local otherwise).
	pub fn rest_sample(self, t: f32) -> CurveSample {
		branch_rest_sample(self.p0, self.p1, self.p2, t)
	}

	/// World-space sample after composing parent attachments and wood stiffness.
	pub fn world_sample(
		self,
		branches: &[BranchDescriptor],
		_plant: &PlantDescriptor,
		t: f32,
		wind_xz: Vec2,
	) -> CurveSample {
		let mut chain = [self; 4];
		let mut hops = 1u32;
		let mut current = self;
		while !current.is_trunk() && hops < MAX_ATTACH_HOPS {
			let Some(parent) = branches.get(current.parent_index as usize).copied() else {
				break;
			};
			chain[hops as usize] = parent;
			hops += 1;
			current = parent;
		}
		let trunk = chain[(hops - 1) as usize];
		let trunk_t = if hops == 1 { t } else { chain[(hops - 2) as usize].parent_t };
		let mut sample = deform_trunk(trunk, trunk_t, wind_xz);
		if hops == 1 {
			return sample;
		}
		for i in (0..hops - 1).rev() {
			let child = chain[i as usize];
			let child_t = if i == 0 { t } else { chain[i as usize - 1].parent_t };
			sample = deform_child(child, child_t, sample, wind_xz);
		}
		sample
	}
}

impl LeafDescriptor {
	pub fn world_base(
		self,
		branches: &[BranchDescriptor],
		plant: &PlantDescriptor,
		wind_xz: Vec2,
	) -> (Vec3, LocalFrame) {
		let Some(parent) = branches.get(self.parent_branch as usize).copied() else {
			return (plant.root, LocalFrame::from_tangent(plant.up, Vec3::Y));
		};
		let sample = parent.world_sample(branches, plant, self.attach_t, wind_xz);
		(sample.position, attachment_frame(sample, self.twist))
	}

	pub fn world_vertex(
		self,
		branches: &[BranchDescriptor],
		plant: &PlantDescriptor,
		wind_xz: Vec2,
		u: f32,
		v: f32,
	) -> Vec3 {
		let (origin, frame) = self.world_base(branches, plant, wind_xz);
		leaf_surface_vertex(origin, frame, self.size_m, u, v, self.fold)
	}
}

fn deform_trunk(branch: BranchDescriptor, t: f32, wind_xz: Vec2) -> CurveSample {
	let rest = branch.rest_sample(t);
	let position = deform_along_curve(rest.position, branch.p0, t, wind_xz, branch.stiffness);
	let rest_hi = branch.rest_sample((t + 0.02).min(1.0));
	let hi = deform_along_curve(rest_hi.position, branch.p0, (t + 0.02).min(1.0), wind_xz, branch.stiffness);
	let tangent = hi - position;
	let frame = vegetation_shaders::wesl::stable_frame(tangent, Vec3::Y);
	CurveSample { position, tangent: frame.along, frame }
}

fn deform_child(
	branch: BranchDescriptor,
	t: f32,
	parent_sample: CurveSample,
	wind_xz: Vec2,
) -> CurveSample {
	let attach = attachment_frame(parent_sample, branch.twist);
	let local = branch.rest_sample(t);
	let rest_world = parent_sample.position
		+ attach.across * local.position.x
		+ attach.along * local.position.y
		+ attach.facing * local.position.z;
	let position =
		deform_along_curve(rest_world, parent_sample.position, t, wind_xz, branch.stiffness);
	let local_hi = branch.rest_sample((t + 0.02).min(1.0));
	let rest_hi = parent_sample.position
		+ attach.across * local_hi.position.x
		+ attach.along * local_hi.position.y
		+ attach.facing * local_hi.position.z;
	let hi = deform_along_curve(
		rest_hi,
		parent_sample.position,
		(t + 0.02).min(1.0),
		wind_xz,
		branch.stiffness,
	);
	let frame = vegetation_shaders::wesl::stable_frame(hi - position, Vec3::Y);
	CurveSample { position, tangent: frame.along, frame }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BushRecipe;
	use crate::topology::BushPlant;
	use bevy::math::Vec3Swizzles;
	use vegetation_shaders::wesl::{wind_field, WindInputs};

	#[test]
	fn child_stays_welded_through_gusts() -> anyhow::Result<()> {
		let plant = BushPlant::generate(5, 0, Vec3::new(3.0, 0.0, -2.0), Vec3::Y, 1.0, &BushRecipe::small_scrub());
		let wind = wind_field(plant.descriptor.root.xz(), WindInputs::botw_gust(1.85));
		for branch in &plant.branches {
			if branch.is_trunk() {
				continue;
			}
			let parent = plant.branches[branch.parent_index as usize];
			let attach = parent.world_sample(
				&plant.branches,
				&plant.descriptor,
				branch.parent_t,
				wind,
			);
			let child_root =
				branch.world_sample(&plant.branches, &plant.descriptor, 0.0, wind);
			assert!(
				(child_root.position - attach.position).length() < 0.06,
				"child detached {} from parent {}",
				(child_root.position - attach.position).length(),
				branch.branch_id
			);
		}
		Ok(())
	}

	#[test]
	fn leaf_bases_stay_on_the_parent_curve() -> anyhow::Result<()> {
		let plant = BushPlant::generate(8, 2, Vec3::ZERO, Vec3::Y, 1.0, &BushRecipe::small_scrub());
		let wind = wind_field(Vec2::ZERO, WindInputs::gentle_botw(0.85));
		for leaf in &plant.leaves {
			let (origin, _) = leaf.world_base(&plant.branches, &plant.descriptor, wind);
			let parent = plant.branches[leaf.parent_branch as usize];
			let sample = parent.world_sample(&plant.branches, &plant.descriptor, leaf.attach_t, wind);
			assert!((origin - sample.position).length() < 1e-3);
		}
		Ok(())
	}

	#[test]
	fn trunk_root_stays_pinned() -> anyhow::Result<()> {
		let plant = BushPlant::generate(1, 0, Vec3::new(-4.0, 1.0, 6.0), Vec3::Y, 1.0, &BushRecipe::leafless_scrub());
		let trunk = plant.branches[0];
		let wind = Vec2::new(3.0, 1.0);
		let root = trunk.world_sample(&plant.branches, &plant.descriptor, 0.0, wind);
		assert!((root.position - plant.descriptor.root).length() < 1e-4);
		Ok(())
	}
}
