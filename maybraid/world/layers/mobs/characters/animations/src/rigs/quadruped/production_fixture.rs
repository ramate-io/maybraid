//! Test-only Epiphant scaffold, checked against the runtime capture's LIVE affines.
//! The fixed midbacks are independent branches omitted by the typed animation tree.
use bevy::math::{Affine3A, Mat3A, Vec3A};
use bevy::prelude::Vec3;
use character_rigs::authoring::BoneId;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

pub(super) fn rig() -> anyhow::Result<QuadrupedV0Rig> {
	let mut rig = QuadrupedV0Rig::for_clip_test();
	let mut rest = rig.binding.effective_rest.clone();
	for name in [
		"anterior_thigh.L",
		"anterior_thigh.R",
		"anterior_shin.L",
		"anterior_shin.R",
		"posterior_thigh.L",
		"posterior_thigh.R",
		"posterior_shin.L",
		"posterior_shin.R",
	] {
		let id = rig
			.binding
			.definition
			.id(name)
			.ok_or_else(|| anyhow::anyhow!("missing {name}"))?;
		rest.local[id.index()].scale = Vec3::new(1.2, 0.95, 1.2);
	}
	rig.binding.refresh_rest(rest);
	rig.pose.copy_from(&rig.binding.effective_rest);
	rig.segment_lengths = rig.binding.metrics.quadruped_leg;
	Ok(rig)
}

pub(super) fn point(rig: &QuadrupedV0Rig, name: &str, local: Vec3) -> anyhow::Result<Vec3> {
	let id = rig
		.binding
		.definition
		.id(name)
		.ok_or_else(|| anyhow::anyhow!("missing {name}"))?;
	Ok(affine(rig, id)?.transform_point3(local))
}

fn affine(rig: &QuadrupedV0Rig, id: BoneId) -> anyhow::Result<Affine3A> {
	let definition = &rig.binding.definition;
	let local = rig
		.pose
		.get(id)
		.ok_or_else(|| anyhow::anyhow!("missing pose {}", id.index()))?
		.compute_affine();
	let name = &definition.names[id.index()];
	let parent = match *name {
		// Exact fixed GLB orientations with Epiphant's 1.35 torso thickness.
		// Each maps local +Z to world +Y; front and rear point opposite ways.
		"upper_back" => Affine3A {
			matrix3: Mat3A::from_cols(
				Vec3A::new(-1.35, 0.0, 0.0),
				Vec3A::new(0.0, 0.0, 1.0),
				Vec3A::new(0.0, 1.35, 0.0),
			),
			translation: Vec3A::ZERO,
		},
		"lumbar" => Affine3A {
			matrix3: Mat3A::from_cols(
				Vec3A::new(1.35, 0.0, 0.0),
				Vec3A::new(0.0, 0.0, -1.0),
				Vec3A::new(0.0, 1.35, 0.0),
			),
			translation: Vec3A::ZERO,
		},
		_ => match definition.parent(id) {
			Some(parent) => affine(rig, parent)?,
			None => Affine3A::IDENTITY,
		},
	};
	Ok(parent * local)
}

#[test]
fn fixed_branches_and_limb_proportions_match_runtime_rest_supports() -> anyhow::Result<()> {
	let rig = rig()?;
	assert!(
		point(&rig, "anterior_thigh.L", Vec3::ZERO)?.distance(Vec3::new(0.675, 0.0, 0.75)) < 1e-5
	);
	assert!(
		point(&rig, "posterior_thigh.L", Vec3::ZERO)?.distance(Vec3::new(0.54, 0.0, -0.75)) < 1e-5
	);
	for name in ["anterior_shin.L", "anterior_shin.R", "posterior_shin.L", "posterior_shin.R"] {
		assert!(
			(point(&rig, name, Vec3::Y * 0.5)?.y + 1.2504375).abs() < 1e-5,
			"{name} support height"
		);
	}
	Ok(())
}
