//! A bounded, explicit world-rendering trial for a fixed natural Storybook grove area.

use bevy::camera::visibility::VisibilitySystems;
use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::query::Allow;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::renderer::{RenderAdapter, RenderDevice};
use bevy::render::RenderApp;
use vegetation_groves::temperate_lower_massives::{
	configure_storybook_field_area, scout_storybook_field_tile, StorybookFieldArea,
	StorybookFieldSample,
};
use vegetation_tree_field::{
	requested_world_tree_mode, supports_world_tree_field, TreeFieldPlugin, TreeFieldSettings,
	TreePatch, TreeRecipe, TreeSample, MAX_BRANCHES_PER_PLANT, MAX_LEAVES_PER_PLANT,
};

#[derive(Resource)]
struct StorybookFieldTrial {
	tile: Vec2,
	area: StorybookFieldArea,
}

pub(super) fn register_world_tree_field(app: &mut App) {
	let mode = requested_world_tree_mode();
	if mode.is_none() && std::env::var("MAYBRAID_EXPERIMENT_TREE_FIELD_SCOUT").as_deref() == Ok("1")
	{
		scout_storybook_field_tile();
		return;
	}
	let tile = std::env::var("MAYBRAID_EXPERIMENT_TREE_FIELD_TILE").ok();
	let Some(tile) = tile.as_deref().and_then(parse_tile) else {
		if mode.is_some() {
			warn!("Tree field trial requires MAYBRAID_EXPERIMENT_TREE_FIELD_TILE=x,z; keeping grove kits");
		}
		return;
	};
	let Some(area) = StorybookFieldArea::from_env() else {
		warn!("Tree field trial expects MAYBRAID_EXPERIMENT_TREE_FIELD_AREA=3x3 or no area; keeping grove kits");
		return;
	};
	if mode.is_none() {
		configure_storybook_field_area(tile, area, false);
		return;
	}
	if std::env::var("MAYBRAID_SHADOWS").as_deref() != Ok("off")
		|| app.get_sub_app(RenderApp).is_none()
	{
		warn!("Tree field trial requires a renderer and shadows off; keeping grove kits");
		return;
	}
	app.add_plugins(TreeFieldPlugin)
		.insert_resource(StorybookFieldTrial { tile, area })
		.add_systems(Startup, admit_tree_field)
		.add_systems(
			PostUpdate,
			publish_placed_trees.after(VisibilitySystems::VisibilityPropagate),
		);
}

fn parse_tile(raw: &str) -> Option<Vec2> {
	let (x, z) = raw.split_once(',')?;
	let tile = Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?);
	tile.is_finite().then_some(tile)
}

fn admit_tree_field(
	trial: Res<StorybookFieldTrial>,
	adapter: Option<Res<RenderAdapter>>,
	device: Option<Res<RenderDevice>>,
	mut settings: ResMut<TreeFieldSettings>,
) {
	let supported = adapter
		.zip(device)
		.is_some_and(|(adapter, device)| supports_world_tree_field(&adapter, &device, trial.area));
	let configured = configure_storybook_field_area(trial.tile, trial.area, supported);
	if !supported || !configured {
		warn!("Tree field trial is unavailable; keeping grove kits");
		return;
	}
	settings.max_plants = trial.area.max_plants();
	settings.max_branches = settings.max_plants * MAX_BRANCHES_PER_PLANT;
	settings.max_leaves = settings.max_plants * MAX_LEAVES_PER_PLANT;
	settings.max_per_lod = settings.max_leaves;
	// Exclude the standalone playground's CPU mirror from both measured field modes.
	settings.diagnostics = false;
	info!(
		target: "maybraid.tree_field",
		tile = ?trial.tile, area = ?trial.area, capacity = settings.max_plants,
		"world trial admitted"
	);
}

fn publish_placed_trees(
	mut commands: Commands,
	sources: Query<
		(Entity, &StorybookFieldSample, &InheritedVisibility, Has<TreePatch>),
		Allow<Disabled>,
	>,
	hierarchy: Query<(Option<&ChildOf>, Option<&Visibility>, Has<Disabled>), Allow<Disabled>>,
	mut owners: Local<HashMap<[u32; 3], (Entity, bool)>>,
) {
	owners.clear();
	for (entity, sample, inherited, patched) in &sources {
		// Root hides precede their deferred descendant disabling. During a warm
		// overlap, both shown levels may contain the same authored plant slot.
		if inherited.get() && lod_ancestry_is_visible(entity, &hierarchy) {
			let owner = owners.entry(sample.slot).or_insert((entity, patched));
			if patched && !owner.1 {
				*owner = (entity, true);
			}
		}
	}
	for (entity, sample, _, patched) in &sources {
		let visible = owners.get(&sample.slot).is_some_and(|owner| owner.0 == entity);
		if visible && !patched {
			let mut recipe = TreeRecipe::storybook();
			recipe.height_m = sample.height..=sample.height;
			commands.entity(entity).insert(TreePatch::new(
				sample.seed,
				recipe,
				1.0,
				vec![TreeSample { root: sample.root, up: sample.up, scale: 1.0 }],
			));
		} else if patched && !visible {
			commands.entity(entity).remove::<TreePatch>();
		}
	}
}

fn lod_ancestry_is_visible(
	mut entity: Entity,
	hierarchy: &Query<(Option<&ChildOf>, Option<&Visibility>, Has<Disabled>), Allow<Disabled>>,
) -> bool {
	for _ in 0..32 {
		let Ok((parent, visibility, disabled)) = hierarchy.get(entity) else { return false };
		if disabled || visibility == Some(&Visibility::Hidden) {
			return false;
		}
		let Some(parent) = parent else { return true };
		entity = parent.parent();
	}
	false
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn exact_finite_tile_coordinate_is_required() {
		assert_eq!(parse_tile("1746.8, 1037.9"), Some(Vec2::new(1746.8, 1037.9)));
		assert!(parse_tile("1,2,3").is_none());
		assert!(parse_tile("1,inf").is_none());
	}

	#[test]
	fn only_visible_storybook_sources_carry_a_patch() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_systems(Update, publish_placed_trees);
		let world = app.world_mut();
		let sample = StorybookFieldSample {
			slot: [123, 456, 0],
			root: Vec3::new(17., 3., 49.),
			up: Vec3::Y,
			height: 15.,
			seed: 72,
		};
		let grove = world.spawn(Visibility::Inherited).id();
		let tree = world.spawn((sample, InheritedVisibility::VISIBLE, ChildOf(grove))).id();
		app.update();
		let patch = app
			.world()
			.get::<TreePatch>(tree)
			.ok_or_else(|| anyhow::anyhow!("missing field patch"))?;
		assert_eq!(patch.seed, sample.seed);
		assert_eq!(patch.samples[0].root, sample.root);
		assert_eq!(patch.recipe.height_m, 15.0..=15.0);
		app.world_mut().entity_mut(grove).insert((Visibility::Hidden, Disabled));
		app.update();
		assert!(app.world().get::<TreePatch>(tree).is_none());
		app.world_mut().entity_mut(tree).insert(Disabled);
		app.world_mut()
			.entity_mut(grove)
			.insert(Visibility::Inherited)
			.remove::<Disabled>();
		app.update();
		assert!(app.world().get::<TreePatch>(tree).is_none());
		app.world_mut().entity_mut(tree).remove::<Disabled>();
		app.update();
		assert!(app.world().get::<TreePatch>(tree).is_some());
		app.world_mut().entity_mut(tree).insert(InheritedVisibility::HIDDEN);
		app.update();
		assert!(app.world().get::<TreePatch>(tree).is_none());
		Ok(())
	}

	#[test]
	fn overlapping_lod_roots_publish_one_patch_per_slot() {
		let mut app = App::new();
		app.add_systems(Update, publish_placed_trees);
		let mut sample = StorybookFieldSample { height: 12.0, ..default() };
		let old_root = app.world_mut().spawn(Visibility::Inherited).id();
		let new_root = app.world_mut().spawn(Visibility::Inherited).id();
		let old = app
			.world_mut()
			.spawn((sample, InheritedVisibility::VISIBLE, ChildOf(old_root)))
			.id();
		app.update();
		let replacement = app
			.world_mut()
			.spawn((sample, InheritedVisibility::VISIBLE, ChildOf(new_root)))
			.id();
		sample.slot[2] = 1;
		let coincident = app
			.world_mut()
			.spawn((sample, InheritedVisibility::VISIBLE, ChildOf(new_root)))
			.id();
		app.update();
		assert!(app.world().get::<TreePatch>(old).is_some());
		assert!(app.world().get::<TreePatch>(replacement).is_none());
		assert!(app.world().get::<TreePatch>(coincident).is_some());
		app.world_mut().entity_mut(old_root).insert((Visibility::Hidden, Disabled));
		app.update();
		assert!(app.world().get::<TreePatch>(old).is_none());
		assert!(app.world().get::<TreePatch>(replacement).is_some());
		assert!(app.world().get::<TreePatch>(coincident).is_some());
	}
}
