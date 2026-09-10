//! Share-then-merge posed kit GLBs into [`scene_ref::MultiSceneMerge`].
//!
//! Same kit mesh + material + [`ParentConfines`] bake into one mesh so instance
//! writes and visibility scale with merged surfaces, not per-tile `Mesh3d`s.
//! Distinct kits stay separate (no unique whole-block merge).

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::LodLazyPending;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
use scene_ref::{MultiSceneMerge, MultiScenePart, SceneRef};

use crate::lod_host_helper::LodHostHelper;
use crate::parent_confines::{confined_scene, ParentConfines};

#[derive(Clone)]
pub(crate) struct KitPart {
	pub scene: SceneRef,
	pub transform: Transform,
	pub material: Option<MaterialRef>,
	pub confines: ParentConfines,
}

struct MergeGroup {
	scene: SceneRef,
	material: Option<MaterialRef>,
	confines: ParentConfines,
	transforms: Vec<Transform>,
}

pub(crate) fn scenes_from_kit_parts(parts: Vec<KitPart>) -> Vec<Box<dyn Scene>> {
	let mut groups: Vec<MergeGroup> = Vec::new();
	for part in parts {
		if let Some(group) = groups.iter_mut().find(|g| {
			g.scene == part.scene && g.material == part.material && g.confines == part.confines
		}) {
			group.transforms.push(part.transform);
		} else {
			groups.push(MergeGroup {
				scene: part.scene,
				material: part.material,
				confines: part.confines,
				transforms: vec![part.transform],
			});
		}
	}
	groups.into_iter().map(emit_group).collect()
}

fn emit_group(group: MergeGroup) -> Box<dyn Scene> {
	let scene = if group.transforms.len() == 1 {
		posed_kit(group.scene, group.transforms[0], group.material)
	} else {
		let parts = group
			.transforms
			.into_iter()
			.map(|transform| MultiScenePart::new(group.scene.clone(), transform));
		merged_kit(MultiSceneMerge::new(parts), group.material)
	};
	match group.confines {
		ParentConfines::External => scene,
		confines => Box::new(confined_scene(confines, scene)),
	}
}

fn posed_kit(
	scene: SceneRef,
	transform: Transform,
	material: Option<MaterialRef>,
) -> Box<dyn Scene> {
	with_optional_material(LodHostHelper::posed_scene_ref_tier(Some(scene), transform), material)
}

fn merged_kit(merge: MultiSceneMerge, material: Option<MaterialRef>) -> Box<dyn Scene> {
	with_optional_material(merge.scene(), material)
}

fn with_optional_material(
	scene: impl Scene + 'static,
	material: Option<MaterialRef>,
) -> Box<dyn Scene> {
	match material {
		Some(material) => Box::new((
			bsn! {
				template_value(MaterialRefRoot(material))
				PropagateToDescendants
				LodLazyPending
			},
			scene,
		)),
		None => Box::new(scene),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use scene_ref::SceneRef;

	fn part(path: &str, x: f32) -> KitPart {
		KitPart {
			scene: SceneRef::glb(path),
			transform: Transform::from_xyz(x, 0.0, 0.0),
			material: None,
			confines: ParentConfines::External,
		}
	}

	#[test]
	fn same_kit_instances_merge_to_one_scene() {
		assert_eq!(
			scenes_from_kit_parts(vec![part("wall.glb", 0.0), part("wall.glb", 1.0)]).len(),
			1
		);
	}

	#[test]
	fn distinct_kits_stay_separate() {
		assert_eq!(
			scenes_from_kit_parts(vec![part("wall.glb", 0.0), part("roof.glb", 0.0)]).len(),
			2
		);
	}

	#[test]
	fn internal_confines_do_not_merge_with_external() {
		let internal = KitPart {
			scene: SceneRef::glb("wall.glb"),
			transform: Transform::IDENTITY,
			material: None,
			confines: ParentConfines::internal(bevy::math::Vec3::ZERO, 4.0),
		};
		assert_eq!(scenes_from_kit_parts(vec![part("wall.glb", 0.0), internal]).len(), 2);
	}
}
