use bevy::asset::RenderAssetUsages;
use bevy::mesh::{
	skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
	PrimitiveTopology, VertexAttributeValues,
};
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceSpawner;
use character_rigs::rigs::humanoid_v0::HumanoidForearmEndpoints;
use std::collections::HashMap;

use crate::assembly::CharacterPartSlot;
use crate::rig::{BoneMap, CharacterPart, CharacterRig, NeedsSkinRemap, PartRigRef};
use crate::skin::remap_part_skin_to_rig;

fn body_mesh(left_y: f32) -> Mesh {
	Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, left_y, 0.0], [0.0, 0.2, 0.0], [0.0, 0.32, 0.0]],
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_JOINT_INDEX,
			VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0]]),
		)
		.with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1.0, 0.0, 0.0, 0.0]; 3])
}

fn body_rig(world: &mut World) -> Entity {
	let left = world.spawn(Name::new("forearm.L")).id();
	let right = world.spawn(Name::new("forearm.R")).id();
	world
		.spawn((
			CharacterRig::default(),
			BoneMap {
				by_name: HashMap::from([("forearm.L".into(), left), ("forearm.R".into(), right)]),
			},
		))
		.id()
}

#[test]
fn body_calibration_tracks_asset_reload_rebinding_and_pending_geometry() -> anyhow::Result<()> {
	let mut world = World::new();
	world.init_resource::<Assets<Mesh>>();
	world.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
	world.init_resource::<Messages<AssetEvent<Mesh>>>();
	world.init_resource::<WorldInstanceSpawner>();
	let mesh = world.resource_mut::<Assets<Mesh>>().add(body_mesh(0.37));
	let binds = world
		.resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
		.add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY; 2]));
	let original_left = world.spawn(Name::new("forearm.L")).id();
	let original_right = world.spawn(Name::new("forearm.R")).id();
	let rig = body_rig(&mut world);
	let part = world
		.spawn((
			CharacterPart { slot: CharacterPartSlot::BodyMesh },
			PartRigRef { rig_root: rig },
			NeedsSkinRemap,
		))
		.id();
	let skin = world
		.spawn((
			ChildOf(part),
			Mesh3d(mesh.clone()),
			SkinnedMesh {
				inverse_bindposes: binds.clone(),
				joints: vec![original_left, original_right],
			},
		))
		.id();
	let mut schedule = Schedule::default();
	schedule.add_systems(remap_part_skin_to_rig);
	let tip = |world: &World, rig| {
		world
			.get::<HumanoidForearmEndpoints>(rig)
			.and_then(|endpoints| endpoints.points[0])
	};

	schedule.run(&mut world);
	assert_eq!(tip(&world, rig), Some(Vec3::Y * 0.37));
	assert!(world.get::<NeedsSkinRemap>(part).is_none());

	// Wardrobe skins cannot replace the actual body calibration.
	let clothing = world
		.spawn((
			CharacterPart { slot: CharacterPartSlot::Clothing },
			PartRigRef { rig_root: rig },
			NeedsSkinRemap,
		))
		.id();
	let clothing_mesh = world.resource_mut::<Assets<Mesh>>().add(body_mesh(0.9));
	world.spawn((
		ChildOf(clothing),
		Mesh3d(clothing_mesh),
		SkinnedMesh { inverse_bindposes: binds, joints: vec![original_left, original_right] },
	));
	schedule.run(&mut world);
	assert_eq!(tip(&world, rig), Some(Vec3::Y * 0.37));

	*world
		.resource_mut::<Assets<Mesh>>()
		.get_mut(&mesh)
		.ok_or_else(|| anyhow::anyhow!("missing mesh"))? = body_mesh(0.28);
	world
		.resource_mut::<Messages<AssetEvent<Mesh>>>()
		.write(AssetEvent::Modified { id: mesh.id() });
	schedule.run(&mut world);
	assert_eq!(tip(&world, rig), None, "modification invalidates the old terminal before retry");
	schedule.run(&mut world);
	assert_eq!(tip(&world, rig), Some(Vec3::Y * 0.28));

	let replacement = world.resource_mut::<Assets<Mesh>>().add(body_mesh(0.41));
	world.entity_mut(skin).insert(Mesh3d(replacement.clone()));
	schedule.run(&mut world);
	schedule.run(&mut world);
	assert_eq!(
		tip(&world, rig),
		Some(Vec3::Y * 0.41),
		"in-place handle changes cannot retain stale geometry"
	);
	let other_rig = body_rig(&mut world);
	world.entity_mut(part).insert(PartRigRef { rig_root: other_rig });
	schedule.run(&mut world);
	schedule.run(&mut world);
	assert_eq!(
		tip(&world, rig),
		None,
		"rebinding a live part clears the old live body's calibration"
	);
	assert_eq!(tip(&world, other_rig), Some(Vec3::Y * 0.41));

	world.resource_mut::<Assets<Mesh>>().remove(replacement.id());
	world
		.resource_mut::<Messages<AssetEvent<Mesh>>>()
		.write(AssetEvent::Removed { id: replacement.id() });
	schedule.run(&mut world);
	schedule.run(&mut world);
	assert_eq!(tip(&world, other_rig), None);
	assert!(world.get::<NeedsSkinRemap>(part).is_some(), "unloaded geometry must remain pending");
	world
		.resource_mut::<Assets<Mesh>>()
		.insert(replacement.id(), body_mesh(0.34))
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	world
		.resource_mut::<Messages<AssetEvent<Mesh>>>()
		.write(AssetEvent::Added { id: replacement.id() });
	schedule.run(&mut world);
	schedule.run(&mut world);
	assert_eq!(tip(&world, other_rig), Some(Vec3::Y * 0.34));
	world.entity_mut(skin).insert(Mesh3d(Handle::default()));
	schedule.run(&mut world);
	schedule.run(&mut world);
	assert_eq!(tip(&world, other_rig), None);
	assert!(
		world.get::<NeedsSkinRemap>(part).is_none(),
		"empty mock handles use an explicit unsupported fallback"
	);
	Ok(())
}

use crate::member::MemberOf;
use crate::skin::fulfill_skin_ref_roots;
use crate::socket::{SkinRefApplied, SkinRefRoot};
use bevy::ecs::system::{RunSystemOnce, System};

fn skin_fixture() -> (App, Entity, Entity, Entity) {
	let mut app = App::new();
	app.set_error_handler(bevy::ecs::error::panic);
	let world = app.world_mut();
	let root = world
		.spawn((crate::CharacterRoot, crate::AssemblyRoot, Transform::IDENTITY))
		.id();
	let rig = world.spawn((CharacterRig::default(), MemberOf(root), ChildOf(root))).id();
	let bone = world.spawn((Name::new("pelvis"), ChildOf(rig))).id();
	let mut map = BoneMap::default();
	map.by_name.insert("pelvis".into(), bone);
	world.entity_mut(rig).insert(map);
	let part = world
		.spawn((CharacterPart::default(), SkinRefRoot::default(), MemberOf(root), ChildOf(root)))
		.id();
	(app, root, rig, part)
}

#[test]
fn fulfillment_resolves_a_live_part_to_its_actual_rig() -> anyhow::Result<()> {
	let (mut app, _, rig, part) = skin_fixture();
	app.world_mut()
		.run_system_once(fulfill_skin_ref_roots)
		.map_err(|error| anyhow::anyhow!("{error}"))?;
	let world = app.world();
	assert_eq!(world.get::<PartRigRef>(part).map(|reference| reference.rig_root), Some(rig));
	assert!(world.get::<NeedsSkinRemap>(part).is_some());
	assert!(world.get::<SkinRefApplied>(part).is_some());
	Ok(())
}

#[test]
fn fulfillment_tolerates_part_retirement_before_deferred_apply() -> anyhow::Result<()> {
	let (mut app, root, rig, part) = skin_fixture();
	let world = app.world_mut();
	let mut fulfill = IntoSystem::into_system(fulfill_skin_ref_roots);
	fulfill.initialize(world);
	fulfill
		.run_without_applying_deferred((), world)
		.map_err(|error| anyhow::anyhow!("{error}"))?;
	assert!(world.get::<PartRigRef>(part).is_none(), "completion must still be deferred");
	// A LOD/assembly retirement may remove this part after skin readiness was
	// observed, while other members of the character remain live.
	assert!(world.despawn(part));
	fulfill.apply_deferred(world);
	assert!(world.get_entity(part).is_err());
	assert!(world.get_entity(root).is_ok());
	assert!(world.get_entity(rig).is_ok());
	Ok(())
}
