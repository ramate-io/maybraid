use super::*;
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
