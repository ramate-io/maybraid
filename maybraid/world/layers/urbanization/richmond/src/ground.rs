//! [`RichmondGround`]: the ground developments are generated over.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use durham::{Durham, Terrain, TerrainMeshBuilder};
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use procedural_common::Bounds2;
use terrain_layer_model::{OnTerrain, TerrainCell, TerrainModel};
use terrain_watersheds::{WaterFill, WaterSurface};

use crate::compose::PadComposable;
use crate::padded::TerrainWithPads;

const SITE_SAMPLE_SIDE: usize = 9;
const SITE_HEIGHT_QUANTILE: f32 = 0.95;

/// A ground model whose cells developments sample and pads compose into.
///
/// Its cells are generated in [`HcsgStorage`] like any other node, so a
/// development reads them as generation dependencies.
pub trait RichmondGround:
	TerrainModel<
	Cell: GroundCell
	          + PadComposable<Padded = TerrainWithPads>
	          + TerrainCell<Mesh = TerrainMeshBuilder>
	          + GenerationScheme<HcsgStorage>,
>
{
	/// Storage group the ground's cells belong to. Surfaces composed from
	/// those cells join it, so a ground rebuild drops them too.
	type Nodes: 'static;
}

impl RichmondGround for OnTerrain<Durham> {
	type Nodes = durham::terrain::DurhamNodes;
}

/// One ground cell's composed surface, as developments sample it.
pub trait GroundCell: TerrainCell {
	/// Composed height (every ground modulation applied) at `(x, z)`.
	fn composed_height_at(&self, x: f32, z: f32) -> f32;

	/// Whether the cell's water overlaps `bounds`.
	fn hydro_overlaps(&self, bounds: Bounds2) -> bool;
}

impl GroundCell for Terrain {
	fn composed_height_at(&self, x: f32, z: f32) -> f32 {
		self.sdf.terrain().height_at_with_all_modulations(x, z)
	}

	fn hydro_overlaps(&self, bounds: Bounds2) -> bool {
		hydro_overlaps_xz(&self.marazion_fills, bounds)
	}
}

/// True when `bounds` overlaps any hydro primitive support in `fills`.
pub fn hydro_overlaps_xz(fills: &[WaterFill], bounds: Bounds2) -> bool {
	for fill in fills {
		match &fill.surface {
			WaterSurface::Hydro { complex } => {
				for node in &complex.hydrology {
					if node.correction_intersects(bounds) {
						return true;
					}
				}
			}
			WaterSurface::Flat { region, .. } => {
				let c = bounds.center();
				if region.sdf(c) <= 0.0 {
					return true;
				}
				let corners = [
					bounds.min,
					Vec2::new(bounds.max.x, bounds.min.y),
					bounds.max,
					Vec2::new(bounds.min.x, bounds.max.y),
				];
				if corners.iter().any(|p| region.sdf(*p) <= 0.0) {
					return true;
				}
			}
		}
	}
	false
}

/// The ground under one development site, as its layout algorithms read it.
pub trait SiteGround {
	fn height_at(&mut self, x: f32, z: f32) -> Option<f32>;

	fn hydro_overlaps(&mut self, bounds: Bounds2) -> bool;

	/// Robust high elevation over a yawed rectangular support.
	///
	/// A dense grid over the complete pad influence catches uphill terrain outside
	/// the flatten core. The 95th percentile sits close to that local high without
	/// letting one narrow terrain spike lift the entire terrace.
	fn height_upper_on_rect(&mut self, center: Vec2, half: Vec2, yaw: f32) -> Option<f32> {
		let (sin, cos) = yaw.sin_cos();
		let mut heights = [0.0; SITE_SAMPLE_SIDE * SITE_SAMPLE_SIDE];
		let mut count = 0;
		for iz in 0..SITE_SAMPLE_SIDE {
			for ix in 0..SITE_SAMPLE_SIDE {
				let u = ix as f32 / (SITE_SAMPLE_SIDE - 1) as f32;
				let v = iz as f32 / (SITE_SAMPLE_SIDE - 1) as f32;
				let local = Vec2::new(-half.x + 2.0 * half.x * u, -half.y + 2.0 * half.y * v);
				let p = center
					+ Vec2::new(cos * local.x + sin * local.y, -sin * local.x + cos * local.y);
				if let Some(h) = self.height_at(p.x, p.y) {
					heights[count] = h;
					count += 1;
				}
			}
		}
		upper_quantile(&mut heights[..count], SITE_HEIGHT_QUANTILE)
	}
}

/// [`SiteGround`] over `G`'s cells in storage.
///
/// Reads the finest stored cell covering each sample and generates `G`'s
/// cell only where nothing covers it, so a site inside the streamed ground
/// never re-tiles it.
pub struct GroundSampler<'a, G: RichmondGround> {
	storage: &'a mut HcsgStorage,
	site: Aabb3d,
	/// Cells overlapping the site, finest first.
	cells: Option<Vec<(Id, Aabb3d)>>,
	_ground: PhantomData<fn() -> G>,
}

impl<'a, G: RichmondGround> GroundSampler<'a, G> {
	pub fn new(storage: &'a mut HcsgStorage, site: Aabb3d) -> Self {
		Self { storage, site, cells: None, _ground: PhantomData }
	}

	fn cells(&mut self) -> &mut Vec<(Id, Aabb3d)> {
		let storage = &*self.storage;
		let site = self.site;
		self.cells.get_or_insert_with(|| {
			let mut cells: Vec<(Id, Aabb3d)> = storage
				.overlapping::<G::Cell>(site)
				.into_iter()
				.filter_map(|id| storage.entry::<G::Cell>(id).map(|entry| (id, entry.bounds)))
				.collect();
			sort_finest_first(&mut cells);
			cells
		})
	}

	fn covering(&mut self, x: f32, z: f32) -> Option<Id> {
		let covers = |bounds: &Aabb3d| {
			x >= bounds.min.x && x <= bounds.max.x && z >= bounds.min.z && z <= bounds.max.z
		};
		if let Some((id, _)) = self.cells().iter().find(|(_, bounds)| covers(bounds)) {
			return Some(*id);
		}
		let probe = Aabb3d::from_min_max(
			bevy::math::Vec3::new(x, self.site.min.y, z),
			bevy::math::Vec3::new(x, self.site.max.y, z),
		);
		let generated = self.generate(probe);
		generated.into_iter().find(|(_, bounds)| covers(bounds)).map(|(id, _)| id)
	}

	/// Generates `G`'s origins in `region` and adds them to the site's cells.
	fn generate(&mut self, region: Aabb3d) -> Vec<(Id, Aabb3d)> {
		let mut generated = Vec::new();
		for OriginalId(id) in self.storage.original_ids_for::<G::Cell>(region) {
			if self.storage.get_or_generate::<G::Cell>(id).is_none() {
				continue;
			}
			if let Some(entry) = self.storage.entry::<G::Cell>(id) {
				generated.push((id, entry.bounds));
			}
		}
		let cells = self.cells();
		cells.extend(generated.iter().copied());
		sort_finest_first(cells);
		generated
	}
}

impl<G: RichmondGround> SiteGround for GroundSampler<'_, G> {
	fn height_at(&mut self, x: f32, z: f32) -> Option<f32> {
		let id = self.covering(x, z)?;
		self.storage.get::<G::Cell>(id).map(|cell| cell.composed_height_at(x, z))
	}

	fn hydro_overlaps(&mut self, bounds: Bounds2) -> bool {
		if self.cells().is_empty() {
			self.generate(self.site);
		}
		let storage = &*self.storage;
		self.cells.iter().flatten().any(|(id, cell)| {
			bounds.min.x <= cell.max.x
				&& bounds.max.x >= cell.min.x
				&& bounds.min.y <= cell.max.z
				&& bounds.max.y >= cell.min.z
				&& storage.get::<G::Cell>(*id).is_some_and(|cell| cell.hydro_overlaps(bounds))
		})
	}
}

fn sort_finest_first(cells: &mut [(Id, Aabb3d)]) {
	cells.sort_by(|(_, a), (_, b)| (a.max.x - a.min.x).total_cmp(&(b.max.x - b.min.x)));
}

fn upper_quantile(values: &mut [f32], quantile: f32) -> Option<f32> {
	if values.is_empty() {
		return None;
	}
	let index = ((values.len() - 1) as f32 * quantile.clamp(0.0, 1.0)).ceil() as usize;
	let (_, value, _) = values.select_nth_unstable_by(index, f32::total_cmp);
	Some(*value)
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use terrain_stamps::{CircleRegion, Region2D};

	/// Level ground at `height`, with water wherever `wet` says.
	pub(crate) struct FlatGround {
		pub height: f32,
		pub wet: fn(Bounds2) -> bool,
	}

	impl SiteGround for FlatGround {
		fn height_at(&mut self, _x: f32, _z: f32) -> Option<f32> {
			Some(self.height)
		}

		fn hydro_overlaps(&mut self, bounds: Bounds2) -> bool {
			(self.wet)(bounds)
		}
	}

	#[test]
	fn flat_fill_overlaps_center() {
		let fills = vec![WaterFill {
			surface: WaterSurface::Flat {
				level: 10.0,
				region: Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 20.0 }),
			},
		}];
		let hit = Bounds2::from_xz(-5.0, -5.0, 5.0, 5.0);
		let miss = Bounds2::from_xz(80.0, 80.0, 90.0, 90.0);
		assert!(hydro_overlaps_xz(&fills, hit));
		assert!(!hydro_overlaps_xz(&fills, miss));
	}

	#[test]
	fn upper_site_height_ignores_one_narrow_spike() -> anyhow::Result<()> {
		let mut heights: Vec<f32> = (0..80).map(|i| i as f32).collect();
		heights.push(1000.0);
		let high = upper_quantile(&mut heights, SITE_HEIGHT_QUANTILE)
			.ok_or_else(|| anyhow::anyhow!("expected a quantile"))?;
		anyhow::ensure!(high >= 70.0, "site height should remain near the local high: {high}");
		anyhow::ensure!(high < 1000.0, "one spike should not lift the terrace: {high}");
		Ok(())
	}
}
