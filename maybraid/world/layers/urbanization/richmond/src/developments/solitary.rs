//! [`SolitaryCell`]: one building on one terrace at the cell center.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use buildings::{Confines, Openings};
use lod::gen::{GenerationScheme, Id, OriginalId};
use lod::hcsg::HcsgStorage;
use procedural_common::{NoiseParams, SeededHash};

use super::site::{DevelopmentKind, DevelopmentSite};
use super::DevelopmentPad;
use crate::artifact::BuiltDevelopment;
use crate::cell::{cell_salt, inscribe_yawed_extents, sample_confines_yaw};
use crate::config::DevelopmentConfig;
use crate::finish::DevelopmentFinish;
use crate::ground::{GroundSampler, RichmondGround, SiteGround};
use crate::pad::{cell_center_xz, PadComplex, PadParams};
use crate::storage::column_bounds;

/// Footprint and height ranges a solitary kind draws its confines from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolitaryEnvelope {
	pub min_footprint: f32,
	pub max_footprint: f32,
	pub min_height: f32,
	pub max_height: f32,
	pub rotates: bool,
}

/// A development kind that fits one building to [`SolitaryPlan`] confines.
pub trait SolitaryKind: 'static {
	const KIND: DevelopmentKind;

	fn envelope() -> SolitaryEnvelope;

	fn finish(hash: SeededHash) -> DevelopmentFinish;

	fn built(plan: &SolitaryPlan, noise: NoiseParams) -> Option<BuiltDevelopment>;
}

/// Terrace, confines, and finish drawn for one solitary cell.
#[derive(Debug, Clone)]
pub struct SolitaryPlan {
	pub cell: Aabb3d,
	pub pad: DevelopmentPad,
	pub confines_height: f32,
	pub confines_extent_xz: Vec2,
	pub confines_yaw: f32,
	pub finish: DevelopmentFinish,
}

impl SolitaryPlan {
	pub fn new<K: SolitaryKind>(cell: Aabb3d, pad_height: f32, config: &DevelopmentConfig) -> Self {
		let hash = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
		let envelope = K::envelope();
		let (min_foot, max_foot) = (envelope.min_footprint, envelope.max_footprint);
		let yaw = if envelope.rotates { sample_confines_yaw(hash.unit(37)) } else { 0.0 };
		let extent_x = min_foot + (max_foot - min_foot) * hash.unit(11);
		let extent_z = min_foot + (max_foot - min_foot) * hash.unit(13);
		let confines_height =
			envelope.min_height + (envelope.max_height - envelope.min_height) * hash.unit(17);
		let confines_extent_xz = inscribe_yawed_extents(extent_x, extent_z, yaw, max_foot);
		Self {
			cell,
			pad: DevelopmentPad {
				height: pad_height,
				complex: PadComplex::building_skirt(
					cell_center_xz(cell),
					confines_extent_xz * 0.5,
					yaw,
					pad_height,
					PadParams::default(),
				),
			},
			confines_height,
			confines_extent_xz,
			confines_yaw: yaw,
			finish: K::finish(hash),
		}
	}

	pub fn center_xz(&self) -> Vec2 {
		cell_center_xz(self.cell)
	}

	/// Unrotated confines AABB sitting on the pad (world space).
	///
	/// Builders author against this axis-aligned box. The sampled yaw is
	/// recorded on [`Confines::roll`] and applied at host spawn about the
	/// cell center.
	pub fn confines_bounds(&self) -> Aabb3d {
		let c = self.center_xz();
		let half = self.confines_extent_xz * 0.5;
		Aabb3d::from_min_max(
			Vec3::new(c.x - half.x, self.pad.height, c.y - half.y),
			Vec3::new(c.x + half.x, self.pad.height + self.confines_height, c.y + half.y),
		)
	}

	/// Fitted confines: unrotated AABB plus yaw on [`Confines::roll`].
	pub fn confines(&self) -> Confines {
		Confines::new(self.confines_bounds(), self.confines_yaw, Openings::new())
	}

	/// World-axis half extents of the yawed building confines.
	pub fn footprint_half_extents(&self) -> Vec2 {
		let half = self.confines_extent_xz * 0.5;
		let (sin, cos) = self.confines_yaw.sin_cos();
		let (sin, cos) = (sin.abs(), cos.abs());
		Vec2::new(cos * half.x + sin * half.y, sin * half.x + cos * half.y)
	}

	/// Replace the terrace with one axis-aligned terrace at the same height.
	pub fn flatten_courtyard(&mut self, half_extents: Vec2, params: PadParams) {
		self.pad.complex = PadComplex::building_skirt(
			self.center_xz(),
			half_extents,
			0.0,
			self.pad.height,
			params,
		);
	}
}

/// A solitary development of kind `K` over ground `G`.
pub struct SolitaryCell<K, G> {
	pub plan: SolitaryPlan,
	_kind: PhantomData<fn() -> (K, G)>,
}

impl<K: SolitaryKind, G> SolitaryCell<K, G> {
	pub fn new(plan: SolitaryPlan) -> Self {
		Self { plan, _kind: PhantomData }
	}

	pub fn built(&self, noise: NoiseParams) -> Option<BuiltDevelopment> {
		K::built(&self.plan, noise)
	}
}

impl<K: SolitaryKind, G: RichmondGround> GenerationScheme<HcsgStorage> for SolitaryCell<K, G> {
	fn original_ids_for(storage: &mut HcsgStorage, region: Aabb3d) -> Vec<OriginalId> {
		DevelopmentSite::ids_of_kind(storage, region, K::KIND)
	}

	/// An authored site sits at its authored height; a procedural one samples
	/// the ground at the cell center and is rejected over water.
	fn build_with_id(storage: &mut HcsgStorage, id: Id) -> Option<(Self, Aabb3d)> {
		let (site, config) = DevelopmentSite::planned(storage, id, K::KIND)?;
		let bounds = column_bounds(site.cell);
		if let Some(authored) = &site.authored {
			return Some((Self::new(SolitaryPlan::new::<K>(site.cell, authored.height, &config)), bounds));
		}
		let mut ground = GroundSampler::<G>::new(storage, bounds);
		let center = cell_center_xz(site.cell);
		let height = ground.height_at(center.x, center.y)?;
		let plan = SolitaryPlan::new::<K>(site.cell, height, &config);
		if ground.hydro_overlaps(plan.pad.complex.bounds) {
			return None;
		}
		Some((Self::new(plan), bounds))
	}
}

#[cfg(test)]
mod tests {
	use material_ref::MaterialId;
	use std::f32::consts::TAU;

	use super::*;
	use crate::cell::{available_footprint, yawed_plan_aabb_extent, DevelopmentExtent};
	use crate::developments::les_halles::LesHallesKind;

	fn les_halles(seed: u32) -> SolitaryPlan {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		SolitaryPlan::new::<LesHallesKind>(cell, 12.0, &DevelopmentConfig { seed, ..Default::default() })
	}

	#[test]
	fn filled_cell_picks_urban_finish() {
		let finish = les_halles(DevelopmentConfig::default().seed).finish;
		assert!(matches!(
			&finish.wall.name,
			MaterialId::Name(n) if n == "stucco" || n == "wood"
		));
		assert!(matches!(
			&finish.roof.name,
			MaterialId::Name(n) if n == "iron" || n == "terracotta" || n == "hay"
		));
	}

	#[test]
	fn filled_cell_samples_continuous_yaw() {
		let eighth = TAU / 8.0;
		let mut off_grid = false;
		for seed in 0..48u32 {
			let plan = les_halles(seed);
			assert!(plan.confines_yaw >= 0.0 && plan.confines_yaw <= TAU + 1e-5);
			let phase = plan.confines_yaw.rem_euclid(eighth);
			if phase > 0.05 && phase < eighth - 0.05 {
				off_grid = true;
			}
			let pad = available_footprint();
			let occupied = yawed_plan_aabb_extent(
				plan.confines_extent_xz.x,
				plan.confines_extent_xz.y,
				plan.confines_yaw,
			);
			assert!(occupied.x <= pad + 1e-3, "yawed AABB x {} exceeds pad {}", occupied.x, pad);
			assert!(occupied.y <= pad + 1e-3, "yawed AABB z {} exceeds pad {}", occupied.y, pad);
			assert!((plan.confines().roll - plan.confines_yaw).abs() < 1e-6);
		}
		assert!(off_grid, "expected at least one heading off the old π/4 lattice");
	}

	#[test]
	fn filled_cell_pad_flattens_the_building_center() {
		let plan = les_halles(DevelopmentConfig::default().seed);
		let c = plan.center_xz();
		assert!((plan.pad.complex.modify_elevation(3.0, c.x, c.y) - 12.0).abs() < 1e-3);
		assert!((plan.pad.complex.modify_elevation(3.0, 400.0, 400.0) - 3.0).abs() < 1e-3);
	}

	#[test]
	fn courtyard_flattens_the_whole_arena_at_the_pad_height() -> anyhow::Result<()> {
		let mut walled = les_halles(42);
		let half = walled.footprint_half_extents() + Vec2::splat(20.0);
		walled.flatten_courtyard(half, PadParams { berm: 0.0, ease: 16.0, round: 0.0 });
		let center = walled.center_xz();
		for corner in [Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0), Vec2::new(1.0, -1.0)] {
			let p = center + corner * (half - Vec2::splat(0.5));
			let y = walled.pad.complex.modify_elevation(-30.0, p.x, p.y);
			anyhow::ensure!((y - 12.0).abs() <= 1e-3, "corner {p} at {y}, expected 12");
		}
		Ok(())
	}
}
