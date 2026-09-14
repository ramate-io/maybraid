//! Shared furniture placement helpers for usage-area fills.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use procedural_common::{aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};
use richmond_building_components::furniture::{FurnitureGeometry, FurnitureNode};
use richmond_building_components::placed::Placement;
use richmond_building_components::{LabelNode, LabelStyle};

use crate::fit::{aabb_xz_extent, Confines, FillRegion, SpaceKind};
use crate::placer::{try_free_extent, try_wall_long, FreeExtentKnobs, WallLongKnobs, WALL_EPS};
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::plan_geom::host_xz;

/// Compact chest that still reads as furniture in leftover gallery pockets.
const RESIDUAL_CHEST: Vec3 = Vec3::new(0.9, 0.75, 0.45);
const MIN_RESIDUAL_FOOTPRINT: f32 = 1.1;
/// One seated place inside a commercial sitting AABB (not a region-filling sofa).
const COMMERCIAL_CHAIR: Vec3 = Vec3::new(0.5, 0.8, 0.5);
const COMMERCIAL_CHAIR_AREA: f32 = 2.2;
const MAX_COMMERCIAL_CHAIRS: usize = 4;
/// Packed counter AABBs are often storey-tall; the kit is one world unit high.
pub const COUNTER_SLOT_HEIGHT: f32 = 1.0;
/// Walkway leftovers get a chest less often than closet / failed-strip pockets.
const WALKWAY_CHEST_RATE: f32 = 0.35;

/// Label + furniture kit pair for one placed AABB.
#[derive(Debug, Clone, PartialEq)]
pub struct FurnitureFill {
	pub label: LabelNode,
	pub furniture: FurnitureNode,
}

/// Build a labeled furniture fill from a packed AABB versus the host volume.
///
/// Stamps abutment, facing yaw, and finish seed from the committed box. Kit
/// reuse: `chair` for seating (livable and commercial sitting areas), `counter`
/// for kitchen runs and stall counters / registers, `chest` for compact
/// bedroom storage, `dresser` / `wardrobe` for desks and bookcases.
pub fn furniture_fill(
	style: LabelStyle,
	text: &str,
	aabb: &Aabb3d,
	host: &Aabb3d,
	roll: f32,
	make: fn(Placement) -> FurnitureNode,
) -> FurnitureFill {
	let mut furniture = make(Placement::IDENTITY);
	let slot = if furniture.geometry == FurnitureGeometry::Counter {
		floor_height_aabb(aabb, COUNTER_SLOT_HEIGHT)
	} else {
		*aabb
	};
	furniture.stamp_host_slot(&slot, host);
	FurnitureFill { label: label_filling_aabb(style, text, aabb, roll), furniture }
}

/// Sit `height` on the floor of `aabb`, keeping the XZ box.
pub fn floor_height_aabb(aabb: &Aabb3d, height: f32) -> Aabb3d {
	let y0 = aabb.min.y;
	Aabb3d::from_min_max(
		Vec3::new(aabb.min.x, y0, aabb.min.z),
		Vec3::new(aabb.max.x, y0 + height.max(1e-4), aabb.max.z),
	)
}

/// Pack chair-sized slots inside a sitting AABB. Does not fill the whole region.
pub fn chairs_in_aabb(
	seating: &Aabb3d,
	roll: f32,
	noise: NoiseParams,
	style: LabelStyle,
	text: &str,
) -> Vec<FurnitureFill> {
	let fp = aabb_xz_extent(seating);
	if fp.x < 1.0 || fp.y < 1.0 {
		return Vec::new();
	}
	let height = (seating.max.y - seating.min.y).min(COMMERCIAL_CHAIR.y).max(0.45);
	let extent = Vec3::new(
		COMMERCIAL_CHAIR.x.min(fp.x * 0.55).max(0.4),
		height,
		COMMERCIAL_CHAIR.z.min(fp.y * 0.55).max(0.4),
	);
	let target =
		((fp.x * fp.y / COMMERCIAL_CHAIR_AREA).floor() as usize).clamp(1, MAX_COMMERCIAL_CHAIRS);
	let cfg = NoiseConfig::new(noise);
	let host = host_xz(seating);
	let mut keep = Vec::new();
	let mut out = Vec::new();
	for i in 0..target {
		let salt = 400 + i as u32;
		let Some(aabb) = try_wall_long(
			seating,
			host,
			&keep,
			&cfg,
			salt,
			WallLongKnobs { extent, wall_eps: WALL_EPS, attempts: 16 },
		)
		.or_else(|| {
			try_free_extent(
				seating,
				host,
				&keep,
				&cfg,
				salt.wrapping_add(17),
				FreeExtentKnobs { extent, prefer_wall: true, wall_eps: WALL_EPS, attempts: 16 },
			)
		}) else {
			break;
		};
		keep.push(aabb3_to_plan(&aabb, PlanAxes::XZ));
		out.push(furniture_fill(style, text, &aabb, seating, roll, FurnitureNode::chair));
	}
	out
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
	matches!(kind, SpaceKind::ExternalSpace | SpaceKind::ClosetSpace | SpaceKind::Walkway)
}

/// Reconstruct confines from a region-filling label (kitchen leftover, etc.).
pub fn confines_from_label(label: &LabelNode, roll: f32) -> Confines {
	let center = label.placement.translation;
	let half = label.placement.scale * 0.5;
	Confines::new(
		Aabb3d::from_min_max(center - half, center + half),
		roll,
		crate::openings::Openings::new(),
	)
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

/// Place a chest in leftover gallery / closet / walkway pockets that can hold one.
///
/// Walkways are occasional (`WALKWAY_CHEST_RATE`). Failed strips and closets
/// always try.
pub fn chests_for_regions(regions: &[FillRegion], noise: NoiseParams) -> Vec<FurnitureFill> {
	let cfg = NoiseConfig::new(noise);
	regions
		.iter()
		.enumerate()
		.filter_map(|(i, region)| {
			if !residual_kind_gets_chest(&region.kind) {
				return None;
			}
			let salt = 200 + i as u32;
			if matches!(region.kind, SpaceKind::Walkway)
				&& cfg.sample_unit_4d(salt as f32, 0.0, 0.0, 11.0) > WALKWAY_CHEST_RATE
			{
				return None;
			}
			chest_in_confines(&region.confines, noise, salt)
		})
		.collect()
}

/// Occasional chest inside an already-labeled leftover (Bites kitchen, etc.).
pub fn occasional_chest_in_confines(
	confines: &Confines,
	noise: NoiseParams,
	salt: u32,
	rate: f32,
) -> Option<FurnitureFill> {
	let cfg = NoiseConfig::new(noise);
	if cfg.sample_unit_4d(salt as f32, 1.0, 0.0, 12.0) > rate.clamp(0.0, 1.0) {
		return None;
	}
	chest_in_confines(confines, noise, salt)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use richmond_building_components::FurnitureGeometry;

	#[test]
	fn sitting_aabb_packs_chairs_not_one_sofa() {
		let seating = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 3.0));
		let chairs = chairs_in_aabb(
			&seating,
			0.0,
			NoiseParams::default(),
			LabelStyle::Green,
			"BitesSeating",
		);
		assert!(!chairs.is_empty());
		assert!(chairs.len() <= MAX_COMMERCIAL_CHAIRS);
		assert!(chairs.iter().all(|fill| fill.furniture.geometry == FurnitureGeometry::Chair));
		for fill in &chairs {
			let e = fill.furniture.placement.scale;
			assert!(e.x < 2.0 && e.z < 2.0, "chair slot should stay chair-sized, got {e:?}");
		}
	}

	#[test]
	fn storey_tall_counter_is_one_metre_and_stays_in_xz() {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(8.0, 3.5, 6.0));
		let band = Aabb3d::from_min_max(Vec3::new(1.0, 0.0, 0.0), Vec3::new(5.0, 3.5, 0.8));
		let fill = furniture_fill(
			LabelStyle::Cyan,
			"BitesCounter",
			&band,
			&host,
			0.0,
			FurnitureNode::counter,
		);
		let s = fill.furniture.placement.scale;
		assert!((s.y - COUNTER_SLOT_HEIGHT).abs() < 1e-3, "counter height {s:?}");
		assert!((s.x - 4.0).abs() < 1e-3, "south-wall along should stay 4m, got {s:?}");
		assert!((s.z - 0.8).abs() < 1e-3, "south-wall depth should stay 0.8m, got {s:?}");
		assert!((fill.furniture.placement.translation.y - 0.5).abs() < 1e-3);
	}

	#[test]
	fn east_wall_counter_keeps_the_packed_box() {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.5, 8.0));
		let band = Aabb3d::from_min_max(Vec3::new(5.2, 0.0, 1.0), Vec3::new(6.0, 3.5, 5.0));
		let fill = furniture_fill(
			LabelStyle::Cyan,
			"BitesCounter",
			&band,
			&host,
			0.0,
			FurnitureNode::counter,
		);
		let s = fill.furniture.placement.scale;
		assert!((s.y - COUNTER_SLOT_HEIGHT).abs() < 1e-3);
		assert!((s.x - 4.0).abs() < 1e-3, "local along is world Z, got {s:?}");
		assert!((s.z - 0.8).abs() < 1e-3, "local depth is world X, got {s:?}");
	}

	#[test]
	fn walkway_residuals_can_get_a_chest() {
		let walk = FillRegion::new(
			SpaceKind::Walkway,
			Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 4.0))),
		);
		let mut any = false;
		for seed in 0..24 {
			let chests =
				chests_for_regions(&[walk.clone()], NoiseParams { seed, ..NoiseParams::default() });
			if !chests.is_empty() {
				any = true;
				assert_eq!(chests[0].furniture.geometry, FurnitureGeometry::Chest);
				break;
			}
		}
		assert!(any, "walkway leftovers should get an occasional chest");
	}
}
