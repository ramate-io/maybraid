//! A bounded, explicit world-rendering trial for a fixed natural Storybook grove area.

mod authored;

use bevy::prelude::*;
use bevy::render::renderer::{RenderAdapter, RenderDevice};
use bevy::render::RenderApp;
use std::sync::Arc;
use vegetation_groves::temperate_lower_massives::{
	configure_storybook_field_area, scout_storybook_field_tile, StorybookFieldArea,
};
use vegetation_tree_field::{
	requested_world_tree_mode, supports_world_tree_field, TreeField, TreeFieldPlugin,
	TreeFieldSettings, TreeGrove, BRANCH_DESCRIPTOR_STRIDE,
};

use crate::ForestGroveTile;

const OWNER_DESCRIPTOR_CAPACITY: u32 = 131_072;

fn owner_settings() -> TreeFieldSettings {
	TreeFieldSettings {
		max_plants: 64,
		max_branches: OWNER_DESCRIPTOR_CAPACITY,
		max_leaves: OWNER_DESCRIPTOR_CAPACITY,
		max_per_lod: OWNER_DESCRIPTOR_CAPACITY,
		diagnostics: false,
		..default()
	}
}

/// Admission is atomic: rejected content keeps its existing CPU visual and collision path.
pub(crate) fn claim_grove_field(tile: &mut ForestGroveTile) -> Option<TreeGrove> {
	let ForestGroveTile::TemperateLowerMassives(grove) = tile else { return None };
	let sources = grove.storybook_field_sources()?;
	let plants = match sources.iter().map(authored::from_source).collect::<Result<Vec<_>, _>>() {
		Ok(plants) => plants,
		Err(reason) => {
			warn!(target: "maybraid.tree_field", reason, "grove retained legacy visuals");
			return None;
		}
	};
	let field = match TreeField::from_plants(plants, &owner_settings()) {
		Ok(field) => field,
		Err(reason) => {
			warn!(target: "maybraid.tree_field", reason, "grove retained legacy visuals");
			return None;
		}
	};
	info!(target: "maybraid.tree_field", min = ?grove.extent.min(),
		plants = field.plants.len(), woody_plants = grove.plants.len(),
		unsupported_plants = grove.plants.len() - field.plants.len(),
		branches = field.branches.len(), leaves = field.leaves.len(),
		"persistent authored grove admitted; Oak and Rory retain legacy visuals");
	grove.claim_storybook_field_visuals();
	Some(TreeGrove(Arc::new(field)))
}

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
			warn!(
				"Tree field trial requires MAYBRAID_EXPERIMENT_TREE_FIELD_TILE=x,z; keeping grove kits"
			);
		}
		return;
	};
	let Some(area) = StorybookFieldArea::from_env() else {
		warn!(
			"Tree field trial expects MAYBRAID_EXPERIMENT_TREE_FIELD_AREA=3x3 or no area; keeping grove kits"
		);
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
		.add_systems(Startup, admit_tree_field);
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
	let supported = adapter.zip(device).is_some_and(|(adapter, device)| {
		supports_world_tree_field(&adapter, &device, trial.area)
			&& device.limits().max_storage_buffer_binding_size
				>= u64::from(OWNER_DESCRIPTOR_CAPACITY) * BRANCH_DESCRIPTOR_STRIDE
			&& device.limits().max_buffer_size
				>= u64::from(OWNER_DESCRIPTOR_CAPACITY) * BRANCH_DESCRIPTOR_STRIDE
			&& device.limits().max_compute_workgroups_per_dimension
				>= OWNER_DESCRIPTOR_CAPACITY.div_ceil(64)
	});
	let configured = configure_storybook_field_area(trial.tile, trial.area, supported);
	if !supported || !configured {
		warn!("Tree field trial is unavailable; keeping grove kits");
		return;
	}
	*settings = owner_settings();
	info!(
		target: "maybraid.tree_field",
		tile = ?trial.tile, area = ?trial.area, capacity = settings.max_plants,
		"world trial admitted"
	);
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
}
