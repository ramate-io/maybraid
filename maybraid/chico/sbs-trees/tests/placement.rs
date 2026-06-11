//! Placement-contract regressions for [#452](https://github.com/ramate-io/maybraid/issues/452).
//!
//! Trees spawn one `Component` root at the caller's transform; constituents are `ChildOf`
//! children carrying **tree-local** transforms. Spawning under a translated + rotated root
//! must therefore compose children to the rotated anchors (the old code stacked Y-only world
//! offsets and ignored the root rotation).

use anyhow::{anyhow, Result};
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use chico_sbs_geometry::DatePalmSbs;
use chico_sbs_trees::{DatePalmStd, PalmBushStd};
use render_item::{CascadeChunk, RenderItem};

const EPS: f32 = 1e-4;

fn spawn_in_world<T: RenderItem>(item: &T, transform: Transform) -> (World, Vec<Entity>) {
	let mut world = World::new();
	world.init_resource::<Assets<Mesh>>();
	let chunk = CascadeChunk::unit_center_chunk();
	let mut queue = CommandQueue::default();
	let roots = {
		let mut commands = Commands::new(&mut queue, &world);
		item.spawn_render_items(&mut commands, &chunk, transform)
	};
	queue.apply(&mut world);
	(world, roots)
}

fn rotated_root() -> Transform {
	Transform::from_translation(Vec3::new(12.0, 3.0, -7.0))
		.with_rotation(Quat::from_rotation_z(0.6))
}

fn child_translations(world: &mut World, root: Entity) -> Result<Vec<Vec3>> {
	let children: Vec<Entity> = world
		.get::<Children>(root)
		.ok_or_else(|| anyhow!("root should have children"))?
		.iter()
		.collect();
	children
		.into_iter()
		.map(|child| {
			world
				.get::<Transform>(child)
				.map(|t| t.translation)
				.ok_or_else(|| anyhow!("child should have a transform"))
		})
		.collect()
}

fn contains_point(translations: &[Vec3], expected: Vec3) -> bool {
	translations.iter().any(|t| t.distance(expected) < EPS)
}

#[test]
fn palm_bush_spawns_single_root_at_caller_transform() -> Result<()> {
	let bush = PalmBushStd::default();
	let transform = rotated_root();
	let (mut world, roots) = spawn_in_world(&bush, transform);

	assert_eq!(roots.len(), 1, "spawn_render_items returns only the root");
	let root = roots[0];

	let root_transform = world
		.get::<Transform>(root)
		.ok_or_else(|| anyhow!("root should have a transform"))?;
	assert!(root_transform.translation.distance(transform.translation) < EPS);
	assert!(root_transform.rotation.angle_between(transform.rotation) < EPS);
	Ok(())
}

#[test]
fn palm_bush_children_carry_tree_local_anchors() -> Result<()> {
	let bush = PalmBushStd::default();
	let transform = rotated_root();
	let (mut world, roots) = spawn_in_world(&bush, transform);
	let root = roots[0];

	let translations = child_translations(&mut world, root)?;
	let ring_count = bush.geometry.crown.ring_count;
	assert_eq!(translations.len() as u32, ring_count + 1, "rings + crown tuft");

	for ring in 0..ring_count {
		let anchor = bush.geometry.crown_ring_position(ring);
		assert!(
			contains_point(&translations, anchor),
			"ring {ring} child should sit at tree-local anchor {anchor}, got {translations:?}"
		);
	}

	let origin = bush.geometry.crown_origin();
	assert!(
		contains_point(&translations, origin),
		"crown tuft should sit at crown origin {origin}, not the raw root translation"
	);

	// Children are tree-local: none of them bake the root's world translation in.
	for t in &translations {
		assert!(
			t.distance(transform.translation) > 1.0,
			"child translation {t} looks world-absolute"
		);
	}
	Ok(())
}

#[test]
fn date_palm_tuft_sits_at_chain_tip_under_rotated_root() -> Result<()> {
	let palm = DatePalmStd::default();
	let transform = rotated_root();
	let (mut world, roots) = spawn_in_world(&palm, transform);

	assert_eq!(roots.len(), 1);
	let root = roots[0];

	let chain = palm.build_chain();
	let tip = DatePalmSbs::trunk_tip_from_chain(&chain);

	let translations = child_translations(&mut world, root)?;
	assert!(
		contains_point(&translations, tip),
		"crown tuft should sit at the tree-local trunk tip {tip}"
	);
	Ok(())
}

#[test]
fn all_palm_bush_entities_are_children_of_the_root() -> Result<()> {
	let bush = PalmBushStd::default();
	let (mut world, roots) = spawn_in_world(&bush, rotated_root());
	let root = roots[0];

	let entities: Vec<Entity> = world
		.query_filtered::<Entity, With<Transform>>()
		.iter(&world)
		.collect();
	for entity in entities {
		if entity == root {
			continue;
		}
		let parent = world
			.get::<ChildOf>(entity)
			.ok_or_else(|| anyhow!("non-root entity should be parented"))?;
		assert_eq!(parent.parent(), root);
	}
	Ok(())
}
