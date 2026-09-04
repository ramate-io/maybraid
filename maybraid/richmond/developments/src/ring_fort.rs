//! Ring fort: a curtain-wall courtyard ring with circular or trazaloid corner keeps.
//!
//! The ring is 2–4 storeys (confines height). Each corner keep is 5–10 storeys
//! and sits on the gallery mass, not off the outer corner. Topology is a star:
//! the ring is the hub, and each keep joins it at a gallery corner.

use std::sync::Arc;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use material_ref::MaterialRef;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::panels::PanelStyle;
use richmond_buildings::{
	Confines, EndCap, FillableRegions, Fit, FitError, LesHallesFloorPlan, Overhang,
	RectangularPitchedRoofComplex, RectangularPitchedRoofComplexParams,
};

use crate::connected::{ConnectedDevelopment, DevelopmentEdge};
use crate::les_halles::MixedUseLesHallesHost;
use crate::placed::BuildingFootprint;

pub use crate::curtain_ring::CurtainRing;
pub use crate::keep::{CircularTower, Keep, RingFortKeep, TrazaloidTower};

/// Historical name for [`RingFortKeep`].
pub type RingFortTower = RingFortKeep;

/// Minimum curtain-wall plan so a deep gallery + courtyard still fit.
const MIN_RING_PLAN: f32 = 80.0;
const TOWER_STOREY_MIN: usize = 5;
const TOWER_STOREY_MAX: usize = 10;
const CIRCULAR_RADIUS_MIN: f32 = 6.0;
const CIRCULAR_RADIUS_MAX: f32 = 12.0;
const TRAZALOID_FOOT_MIN: f32 = 12.0;
const TRAZALOID_FOOT_MAX: f32 = 20.0;

const SALT_KIND: f32 = 53.0;
const SALT_FLOORS: f32 = 59.0;
const SALT_SIZE: f32 = 61.0;

const CORNERS: [(f32, f32); 4] = [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)];

/// Corner join from a keep onto the courtyard ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RingFortJoin;

/// One site in a [`RingFort`]: the courtyard curtain or a corner keep.
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortSite {
	Ring(Box<CurtainRing>),
	Keep(RingFortKeep),
}

/// Sibling LOD host emitted by [`RingFort`].
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortHost {
	Ring(Box<MixedUseLesHallesHost>),
	Circular(Arc<CircularTower>),
	Trazaloid(Arc<TrazaloidTower>),
	KeepStairwell(richmond_buildings::ConnectingStairwell),
	GalleryRoof(RectangularPitchedRoofComplex),
}

/// Courtyard ring with four corner keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct RingFort {
	pub connected: ConnectedDevelopment<RingFortSite, RingFortJoin>,
	pub roof: RectangularPitchedRoofComplex,
}

impl RingFort {
	pub fn ring(&self) -> Option<&CurtainRing> {
		self.connected.nodes.iter().find_map(|site| match site {
			RingFortSite::Ring(ring) => Some(ring.as_ref()),
			RingFortSite::Keep(_) => None,
		})
	}

	pub fn keeps(&self) -> impl Iterator<Item = &RingFortKeep> {
		self.connected.nodes.iter().filter_map(|site| match site {
			RingFortSite::Keep(keep) => Some(keep),
			RingFortSite::Ring(_) => None,
		})
	}

	pub fn towers(&self) -> impl Iterator<Item = &RingFortKeep> {
		self.keeps()
	}

	pub fn hosts(&self) -> Vec<RingFortHost> {
		let mut out = Vec::new();
		for site in &self.connected.nodes {
			match site {
				RingFortSite::Ring(ring) => {
					out.extend(
						ring.hosts_without_roof()
							.into_iter()
							.map(|host| RingFortHost::Ring(Box::new(host))),
					);
				}
				RingFortSite::Keep(RingFortKeep::Circular(keep)) => {
					out.push(RingFortHost::Circular(Arc::new(keep.shell.clone())));
					out.extend(keep.stairwells.iter().cloned().map(RingFortHost::KeepStairwell));
				}
				RingFortSite::Keep(RingFortKeep::Trazaloid(keep)) => {
					out.push(RingFortHost::Trazaloid(Arc::new(keep.shell.clone())));
					out.extend(keep.stairwells.iter().cloned().map(RingFortHost::KeepStairwell));
				}
			}
		}
		out.push(RingFortHost::GalleryRoof(self.roof.clone()));
		out
	}

	pub fn with_finish(mut self, wall: MaterialRef, roof: MaterialRef) -> Self {
		for site in &mut self.connected.nodes {
			match site {
				RingFortSite::Ring(ring) => {
					**ring = ring.as_ref().clone().with_finish(wall.clone(), roof.clone());
				}
				RingFortSite::Keep(keep) => {
					*keep = keep.clone().with_wall_material(wall.clone());
				}
			}
		}
		self.roof = self.roof.clone().with_surface_material(roof);
		self
	}
}

impl BuildingFootprint for RingFort {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		let mut rects = Vec::new();
		if let Some(ring) = self.ring() {
			rects.extend(ring.footprint_rects());
		}
		for keep in self.keeps() {
			let c = keep.center_xz();
			let half = keep.plan_half_extent();
			rects.push(Aabb2d::new(Vec2::new(c.x, c.z), Vec2::splat(half)));
		}
		rects
	}
}

impl Fit for RingFort {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let extent = Vec3::from(confines.bounds.max - confines.bounds.min);
		if extent.x + 1e-3 < MIN_RING_PLAN || extent.z + 1e-3 < MIN_RING_PLAN {
			return Err(FitError::TooSmall { reason: "ring_fort_ring" });
		}

		let (ring, _) = CurtainRing::fit(confines, noise)?;
		let last = ring.last_plan().ok_or(FitError::TooSmall { reason: "ring_fort_storeys" })?;
		let max_half = (ring.gallery_width() * 0.48).max(4.0);
		let cfg = NoiseConfig::new(noise);
		let center = confines.center();

		let mut keeps = Vec::with_capacity(4);
		for (i, &(sx, sz)) in CORNERS.iter().enumerate() {
			let origin = ring
				.keep_anchor(sx, sz)
				.ok_or(FitError::TooSmall { reason: "ring_fort_storeys" })?;
			let spec = sample_keep_spec(&cfg, center, i, max_half);
			keeps.push(spec.build(origin, (sx, sz)));
		}
		let corner_half = [
			keeps[0].plan_half_extent(),
			keeps[1].plan_half_extent(),
			keeps[2].plan_half_extent(),
			keeps[3].plan_half_extent(),
		];
		let roof = gallery_roof(last, corner_half);

		let mut nodes = Vec::with_capacity(5);
		nodes.push(RingFortSite::Ring(Box::new(ring)));
		let mut edges = Vec::with_capacity(4);
		for keep in keeps {
			let keep_i = nodes.len();
			nodes.push(RingFortSite::Keep(keep));
			edges.push(DevelopmentEdge::new(0, keep_i, RingFortJoin));
		}

		Ok((
			Self { connected: ConnectedDevelopment::new(confines.bounds, nodes, edges), roof },
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

struct KeepSpec {
	circular: bool,
	floors: usize,
	size: f32,
}

impl KeepSpec {
	fn build(&self, origin: Vec3, corner: (f32, f32)) -> RingFortKeep {
		if self.circular {
			RingFortKeep::circular(origin, self.size, self.floors)
		} else {
			RingFortKeep::trazaloid(origin, self.size, self.floors, corner)
		}
	}
}

fn sample_keep_spec(cfg: &NoiseConfig, center: Vec3, index: usize, max_half: f32) -> KeepSpec {
	let salt = index as f32;
	let circular = cfg.sample_unit_4d(center.x, center.y, center.z, SALT_KIND + salt) < 0.5;
	let floors = TOWER_STOREY_MIN
		+ cfg.sample_range_usize_4d(
			0,
			TOWER_STOREY_MAX - TOWER_STOREY_MIN + 1,
			center.x,
			center.y,
			center.z,
			SALT_FLOORS + salt,
		);
	if circular {
		let lo = CIRCULAR_RADIUS_MIN.min(max_half);
		let hi = CIRCULAR_RADIUS_MAX.min(max_half).max(lo);
		let radius =
			cfg.sample_range_f32_4d(lo, hi, center.x, center.y, center.z, SALT_SIZE + salt);
		KeepSpec { circular: true, floors, size: radius }
	} else {
		let max_foot = (max_half * 2.0).max(TRAZALOID_FOOT_MIN);
		let lo = TRAZALOID_FOOT_MIN.min(max_foot);
		let hi = TRAZALOID_FOOT_MAX.min(max_foot).max(lo);
		let foot = cfg.sample_range_f32_4d(lo, hi, center.x, center.y, center.z, SALT_SIZE + salt);
		KeepSpec { circular: false, floors, size: foot }
	}
}

/// Four gallery pitches from outer wall to courtyard, shortened so corner keeps sit in holes.
fn gallery_roof(plan: &LesHallesFloorPlan, corner_half: [f32; 4]) -> RectangularPitchedRoofComplex {
	let cx = plan.center_xz.x;
	let cz = plan.center_xz.z;
	let y0 = plan.center_xz.y + plan.storey_height;
	let ox = plan.outer.x * 0.5;
	let oz = plan.outer.y * 0.5;
	let ix = plan.courtyard.x * 0.5;
	let iz = plan.courtyard.y * 0.5;
	let gx = (ox - ix).max(0.0);
	let gz = (oz - iz).max(0.0);
	let rise = (gx.min(gz) * 0.55).clamp(2.2, 4.5);
	let y1 = y0 + rise;

	let clamp_inset =
		|half: f32, gallery: f32, rim: f32| (gallery * 0.5 + half).min((rim - 1.2).max(0.0));
	let gw = plan.parameterized.gallery_width;
	let ne_x = clamp_inset(corner_half[0], gw, gx);
	let ne_z = clamp_inset(corner_half[0], gw, gz);
	let nw_x = clamp_inset(corner_half[1], gw, gx);
	let nw_z = clamp_inset(corner_half[1], gw, gz);
	let se_x = clamp_inset(corner_half[2], gw, gx);
	let se_z = clamp_inset(corner_half[2], gw, gz);
	let sw_x = clamp_inset(corner_half[3], gw, gx);
	let sw_z = clamp_inset(corner_half[3], gw, gz);

	let mut volumes = Vec::new();
	let mut push = |min_x: f32, max_x: f32, min_z: f32, max_z: f32| {
		if max_x - min_x > 0.4 && max_z - min_z > 0.4 {
			volumes.push(Aabb3d::from_min_max(
				Vec3::new(min_x, y0, min_z),
				Vec3::new(max_x, y1, max_z),
			));
		}
	};
	push(cx - ox + nw_x, cx + ox - ne_x, cz + iz, cz + oz);
	push(cx - ox + sw_x, cx + ox - se_x, cz - oz, cz - iz);
	push(cx - ox, cx - ix, cz - oz + sw_z, cz + oz - nw_z);
	push(cx + ix, cx + ox, cz - oz + se_z, cz + oz - ne_z);

	RectangularPitchedRoofComplexParams::new(volumes)
		.overhang(Overhang::Fixed(0.45))
		.end_cap(EndCap::Hip)
		.style(PanelStyle::ShepherdsThatch)
		.build()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fort_bounds() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-80.0, 0.0, -80.0), Vec3::new(80.0, 12.0, 80.0))
	}

	fn fit_fort(seed: i32) -> anyhow::Result<RingFort> {
		let confines = Confines::from_bounds(fort_bounds());
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		RingFort::fit_to_confines(&confines, noise)
			.map(|(fort, _)| fort)
			.map_err(|err| anyhow::anyhow!("ring fort fit failed: {err:?}"))
	}

	#[test]
	fn ring_is_two_to_four_storeys_with_four_taller_keeps() -> anyhow::Result<()> {
		let fort = fit_fort(42)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let ring_floors = ring.tower.floor_count();
		anyhow::ensure!((2..=4).contains(&ring_floors), "ring storeys {ring_floors} outside 2..=4");
		let keeps: Vec<_> = fort.keeps().collect();
		anyhow::ensure!(keeps.len() == 4, "expected 4 corner keeps, got {}", keeps.len());
		for keep in &keeps {
			let n = keep.storey_count();
			anyhow::ensure!((5..=10).contains(&n), "keep storeys {n} outside 5..=10");
			anyhow::ensure!(n > ring_floors, "keep should out-climb the ring");
			anyhow::ensure!(!keep.stairwells().is_empty(), "keep needs stairwells");
		}
		anyhow::ensure!(fort.connected.edges.len() == 4);
		anyhow::ensure!(fort.connected.topology_is_valid());
		Ok(())
	}

	#[test]
	fn keeps_sit_on_the_gallery_not_off_the_corner() -> anyhow::Result<()> {
		let fort = fit_fort(11)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let last = ring.last_plan().ok_or_else(|| anyhow::anyhow!("ring has no storey"))?;
		let gallery = last.parameterized.gallery_width;
		let hx = last.outer.x * 0.5 - gallery * 0.5;
		let hz = last.outer.y * 0.5 - gallery * 0.5;
		let eave_y = last.center_xz.y + last.storey_height;
		let outer_hx = last.outer.x * 0.5;
		let outer_hz = last.outer.y * 0.5;
		for keep in fort.keeps() {
			let c = keep.center_xz();
			let half = keep.plan_half_extent();
			let dx = (c.x - last.center_xz.x).abs();
			let dz = (c.z - last.center_xz.z).abs();
			anyhow::ensure!((dx - hx).abs() < 1e-3, "keep x {dx} vs gallery-corner {hx}");
			anyhow::ensure!((dz - hz).abs() < 1e-3, "keep z {dz} vs gallery-corner {hz}");
			anyhow::ensure!((c.y - eave_y).abs() < 1e-3, "keep y {} vs eave {eave_y}", c.y);
			anyhow::ensure!(
				(c.x - last.center_xz.x).abs() + half <= outer_hx + 0.25,
				"keep should stay on the outer footprint (x)"
			);
			anyhow::ensure!(
				(c.z - last.center_xz.z).abs() + half <= outer_hz + 0.25,
				"keep should stay on the outer footprint (z)"
			);
		}
		Ok(())
	}

	#[test]
	fn hosts_include_ring_keeps_stairs_and_gallery_roof() -> anyhow::Result<()> {
		let fort = fit_fort(7)?;
		let hosts = fort.hosts();
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::Ring(_))));
		anyhow::ensure!(hosts
			.iter()
			.any(|h| { matches!(h, RingFortHost::Circular(_) | RingFortHost::Trazaloid(_)) }));
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::KeepStairwell(_))));
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::GalleryRoof(_))));
		anyhow::ensure!(!hosts.iter().any(|h| matches!(
			h,
			RingFortHost::Ring(inner) if matches!(inner.as_ref(), MixedUseLesHallesHost::Roof(_))
		)));
		Ok(())
	}

	fn xz_covers(volume: &Aabb3d, p: Vec3, pad: f32) -> bool {
		p.x > volume.min.x + pad
			&& p.x < volume.max.x - pad
			&& p.z > volume.min.z + pad
			&& p.z < volume.max.z - pad
	}

	#[test]
	fn gallery_roof_covers_ring_not_courtyard_or_keeps() -> anyhow::Result<()> {
		let fort = fit_fort(19)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let last = ring.last_plan().ok_or_else(|| anyhow::anyhow!("ring has no storey"))?;
		let volumes = &fort.roof.params().volumes;
		anyhow::ensure!(volumes.len() == 4, "expected 4 gallery bars, got {}", volumes.len());

		let court = last.center_xz;
		anyhow::ensure!(
			!volumes.iter().any(|v| xz_covers(v, court, 0.05)),
			"gallery roof covers courtyard center"
		);

		let gallery = Vec3::new(
			last.center_xz.x,
			0.0,
			last.center_xz.z + (last.outer.y + last.courtyard.y) * 0.25,
		);
		anyhow::ensure!(
			volumes.iter().any(|v| xz_covers(v, gallery, 0.05)),
			"north gallery should be roofed"
		);

		for keep in fort.keeps() {
			let c = keep.center_xz();
			anyhow::ensure!(
				!volumes.iter().any(|v| xz_covers(v, c, 0.05)),
				"roof covers keep at {:?}",
				c
			);
		}
		Ok(())
	}

	#[test]
	fn with_finish_shades_keeps_and_gallery_roof() -> anyhow::Result<()> {
		use lod::gen::LodSceneLevel;
		use material_ref::MaterialId;
		use richmond_building_components::BuildingComponents;

		let wall = MaterialRef::named("stucco");
		let roof = MaterialRef::named("iron");
		let mut saw_circular = false;
		let mut saw_trazaloid = false;
		for seed in 0..24 {
			let painted = fit_fort(seed)?.with_finish(wall.clone(), roof.clone());
			anyhow::ensure!(
				!painted.roof.roofs().is_empty()
					&& painted.roof.roofs().iter().all(|pitch| {
						matches!(
							pitch.surface_material().map(|m| &m.name),
							Some(MaterialId::Name(n)) if n == "iron"
						)
					}),
				"gallery pitches should carry the roof look"
			);
			for keep in painted.keeps() {
				match keep {
					RingFortKeep::Circular(keep) => {
						saw_circular = true;
						let nodes =
							keep.shell.partition_nodes_for_level(LodSceneLevel::High).flatten();
						anyhow::ensure!(
							nodes.iter().any(|n| {
								matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco")
							}),
							"circular keep partitions should carry the wall look"
						);
					}
					RingFortKeep::Trazaloid(keep) => {
						saw_trazaloid = true;
						let nodes = keep.shell.panel_nodes_for_level(LodSceneLevel::High).flatten();
						anyhow::ensure!(
							nodes.iter().any(|n| {
								matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco")
							}),
							"trazaloid keep panels should carry the wall look"
						);
					}
				}
			}
			if saw_circular && saw_trazaloid {
				break;
			}
		}
		anyhow::ensure!(saw_circular, "expected a shaded circular keep");
		anyhow::ensure!(saw_trazaloid, "expected a shaded trazaloid keep");
		Ok(())
	}

	#[test]
	fn storey_counts_vary_across_seeds() -> anyhow::Result<()> {
		let mut ring_counts = std::collections::BTreeSet::new();
		let mut saw_circular = false;
		let mut saw_trazaloid = false;
		for seed in 0..24 {
			let fort = fit_fort(seed)?;
			let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing ring"))?;
			ring_counts.insert(ring.tower.floor_count());
			for keep in fort.keeps() {
				match keep {
					RingFortKeep::Circular(_) => saw_circular = true,
					RingFortKeep::Trazaloid(_) => saw_trazaloid = true,
				}
			}
		}
		anyhow::ensure!(ring_counts.len() >= 2, "ring storeys should vary, got {ring_counts:?}");
		anyhow::ensure!(saw_circular, "expected at least one circular keep");
		anyhow::ensure!(saw_trazaloid, "expected at least one trazaloid keep");
		Ok(())
	}

	#[test]
	fn curtain_gallery_is_wide_enough_for_keeps() -> anyhow::Result<()> {
		let fort = fit_fort(5)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing ring"))?;
		let gallery = ring.gallery_width();
		anyhow::ensure!(gallery + 1e-3 >= 16.0, "curtain gallery {gallery:.1} too thin");
		for keep in fort.keeps() {
			anyhow::ensure!(
				keep.plan_half_extent() * 2.0 <= gallery + 0.5,
				"keep diameter {} vs gallery {gallery}",
				keep.plan_half_extent() * 2.0
			);
		}
		Ok(())
	}
}
