//! A bounded, explicit world-rendering trial for one natural Storybook grove tile.

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::render::renderer::{RenderAdapter, RenderDevice};
use bevy::render::RenderApp;
use vegetation_groves::temperate_lower_massives::{
	configure_storybook_field_tile, StorybookFieldSample, MAX_STORYBOOK_FIELD_TREES,
};
use vegetation_tree_field::{
	requested_world_tree_mode, supports_world_tree_field, TreeFieldPlugin, TreeFieldSettings,
	TreePatch, TreeRecipe, TreeSample, MAX_LEAVES_PER_PLANT,
};

#[derive(Resource)]
struct StorybookFieldTrial(Vec2);

pub(super) fn register_world_tree_field(app: &mut App) {
	let mode = requested_world_tree_mode();
	let tile = std::env::var("MAYBRAID_EXPERIMENT_TREE_FIELD_TILE").ok();
	let Some(tile) = tile.as_deref().and_then(parse_tile) else {
		if mode.is_some() {
			warn!("Tree field trial requires MAYBRAID_EXPERIMENT_TREE_FIELD_TILE=x,z; keeping grove kits");
		}
		return;
	};
	if mode.is_none() {
		configure_storybook_field_tile(tile, false);
		return;
	}
	if std::env::var("MAYBRAID_SHADOWS").as_deref() != Ok("off")
		|| app.get_sub_app(RenderApp).is_none()
	{
		warn!("Tree field trial requires a renderer and shadows off; keeping grove kits");
		return;
	}
	app.add_plugins(TreeFieldPlugin)
		.insert_resource(StorybookFieldTrial(tile))
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
		.is_some_and(|(adapter, device)| supports_world_tree_field(&adapter, &device));
	let configured = configure_storybook_field_tile(trial.0, supported);
	if !supported || !configured {
		warn!("Tree field trial is unavailable; keeping grove kits");
		return;
	}
	settings.max_per_lod = MAX_STORYBOOK_FIELD_TREES as u32 * MAX_LEAVES_PER_PLANT;
	// Exclude the standalone playground's CPU mirror from both measured field modes.
	settings.diagnostics = false;
	info!(target: "maybraid.tree_field", tile = ?trial.0, "world trial admitted");
}

fn publish_placed_trees(
	mut commands: Commands,
	sources: Query<
		(Entity, &StorybookFieldSample, &InheritedVisibility, Has<TreePatch>),
		Or<(Added<StorybookFieldSample>, Changed<InheritedVisibility>)>,
	>,
) {
	for (entity, sample, visible, patched) in &sources {
		if visible.get() && !patched {
			let mut recipe = TreeRecipe::storybook();
			recipe.height_m = sample.height..=sample.height;
			commands.entity(entity).insert(TreePatch::new(
				sample.seed,
				recipe,
				1.0,
				vec![TreeSample { root: sample.root, up: sample.up, scale: 1.0 }],
			));
		} else if patched && !visible.get() {
			commands.entity(entity).remove::<TreePatch>();
		}
	}
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
			root: Vec3::new(17., 3., 49.),
			up: Vec3::Y,
			height: 15.,
			seed: 72,
		};
		let tree = world.spawn((sample, InheritedVisibility::VISIBLE)).id();
		app.update();
		let patch = app
			.world()
			.get::<TreePatch>(tree)
			.ok_or_else(|| anyhow::anyhow!("missing field patch"))?;
		assert_eq!(patch.seed, sample.seed);
		assert_eq!(patch.samples[0].root, sample.root);
		assert_eq!(patch.recipe.height_m, 15.0..=15.0);
		app.world_mut().entity_mut(tree).insert(InheritedVisibility::HIDDEN);
		app.update();
		assert!(app.world().get::<TreePatch>(tree).is_none());
		Ok(())
	}
}
