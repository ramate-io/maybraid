//! Shared furniture placement helpers for usage-area fills.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::placed::Placement;
use richmond_building_components::{LabelNode, LabelStyle};

use crate::fit::{aabb_xz_extent, Confines, FillRegion, SpaceKind};
use crate::placer::{try_free_extent, try_wall_long, FreeExtentKnobs, WallLongKnobs, WALL_EPS};
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::plan_geom::host_xz;

/// Compact chest that still reads as furniture in leftover gallery pockets.
const RESIDUAL_CHEST: Vec3 = Vec3::new(0.9, 0.75, 0.45);
const MIN_RESIDUAL_FOOTPRINT: f32 = 1.1;

/// Label + furniture kit pair for one placed AABB.
#[derive(Debug, Clone, PartialEq)]
pub struct FurnitureFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Build a labeled furniture fill from a packed AABB versus the host volume.
///
/// Stamps abutment, facing yaw, and finish seed from the committed box. Kit
/// reuse: `chair` for seating, `counter` for kitchen runs, `chest` for compact
/// bedroom storage, `dresser` / `wardrobe` for desks and bookcases.
pub fn furniture_fill(
	style: LabelStyle,
	text: &str,
	aabb: &Aabb3d,
	host: &Aabb3d,
	roll: f32,
	make: fn(Placement) -> FurnitureNode,
) -> FurnitureFill {
	let mut furniture = make(placement_filling_aabb(aabb));
	furniture.stamp_host_slot(aabb, host);
	FurnitureFill { label: label_filling_aabb(style, text, aabb, roll), furniture }
}

/// Placement that fills `aabb` with a unit cube centered in the volume.
///
/// Yaw is identity here; [`FurnitureNode::stamp_host_slot`] writes facing.
pub fn placement_filling_aabb(aabb: &Aabb3d) -> Placement {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let extent = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	Placement::new(center, 0.0).with_scale(extent)
}

/// Leftover pockets that should still get a painted chest.
pub fn residual_kind_gets_chest(kind: &SpaceKind) -> bool {
	matches!(kind, SpaceKind::ExternalSpace | SpaceKind::ClosetSpace)
}

/// Re-tag nested internal leftovers as closet pockets (not stair shafts).
pub fn as_closet_if_internal(region: FillRegion) -> FillRegion {
	match region.kind {
		SpaceKind::InternalSpace => FillRegion::new(SpaceKind::ClosetSpace, region.confines),
		_ => region,
	}
}

/// One wall-hugging chest inside `confines`, or `None` if the pocket is too tight.
pub fn chest_in_confines(
	confines: &Confines,
	noise: NoiseParams,
	salt: u32,
) -> Option<FurnitureFill> {
	let host3 = &confines.bounds;
	let fp = aabb_xz_extent(host3);
	if fp.x < MIN_RESIDUAL_FOOTPRINT || fp.y < MIN_RESIDUAL_FOOTPRINT {
		return None;
	}
	let height = (host3.max.y - host3.min.y).min(RESIDUAL_CHEST.y).max(0.4);
	let extent = Vec3::new(
		RESIDUAL_CHEST.x.min(fp.x * 0.45).max(0.5),
		height,
		RESIDUAL_CHEST.z.min(fp.y.min(fp.x) * 0.4).max(0.35),
	);
	let cfg = NoiseConfig::new(noise);
	let host = host_xz(host3);
	let aabb = try_wall_long(
		host3,
		host,
		&[],
		&cfg,
		salt,
		WallLongKnobs { extent, wall_eps: WALL_EPS, attempts: 16 },
	)
	.or_else(|| {
		try_free_extent(
			host3,
			host,
			&[],
			&cfg,
			salt.wrapping_add(17),
			FreeExtentKnobs { extent, prefer_wall: true, wall_eps: WALL_EPS, attempts: 16 },
		)
	})?;
	Some(furniture_fill(
		LabelStyle::Gray,
		"Chest",
		&aabb,
		host3,
		confines.roll,
		FurnitureNode::chest,
	))
}

/// Place a chest in every leftover gallery / closet pocket that can hold one.
pub fn chests_for_regions(regions: &[FillRegion], noise: NoiseParams) -> Vec<FurnitureFill> {
	regions
		.iter()
		.enumerate()
		.filter_map(|(i, region)| {
			residual_kind_gets_chest(&region.kind)
				.then(|| chest_in_confines(&region.confines, noise, 200 + i as u32))
				.flatten()
		})
		.collect()
}
