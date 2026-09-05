//! Forest world source: overlapping developments modulate height and punch flatten/grade.

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::Vec3;
use chico_forests::{ChicoGrove, GroveWorldSource};
use chico_groves::GroveWorldSample;
use lod::gen::{GeneratingSpatialIndex, GenerationScheme, OriginalId};
use lod::lod_ref::LodRef;
use richmond_development_models::{pad::PadStage, DevelopmentCell, DevelopmentIndex, PadComplex};

/// Present forest groves against an inner world sample after pad marshalling.
#[derive(SystemParam)]
pub struct DevelopmentExclusions<'w, 's, Inner: SystemParam + 'static> {
	inner: StaticSystemParam<'w, 's, Inner>,
	development: DevelopmentIndex<'w>,
}

impl<Inner: SystemParam + 'static> GroveWorldSource for DevelopmentExclusions<'_, '_, Inner>
where
	for<'a, 'b> Inner::Item<'a, 'b>: GroveWorldSource,
{
	fn sample(
		&mut self,
		grove: &ChicoGrove,
		lod_ref: &LodRef,
	) -> Option<impl GroveWorldSample + Clone + Send + Sync + 'static> {
		let bounds = grove.aabb();
		let ids = <DevelopmentCell as GenerationScheme<DevelopmentIndex<'_>>>::original_ids_for(
			&mut self.development,
			bounds,
		);
		for OriginalId(development_id) in ids {
			let _ = GeneratingSpatialIndex::<DevelopmentCell>::get_or_generate(
				&mut self.development,
				development_id,
				lod_ref,
			);
		}
		let pads = self.development.store.merged_pad_complex(bounds);
		let base = self.inner.sample(grove, lod_ref)?;
		Some(DevelopmentGroveSample { base, pads })
	}
}

/// Inner height field with pad terraces and flatten/grade planting holes.
#[derive(Clone)]
pub struct DevelopmentGroveSample<Base> {
	base: Base,
	pads: PadComplex,
}

impl<Base: GroveWorldSample> GroveWorldSample for DevelopmentGroveSample<Base> {
	fn height_at(&self, position: Vec3) -> f32 {
		self.pads
			.modify_elevation(self.base.height_at(position), position.x, position.z)
	}

	fn steepness_at(&self, position: Vec3) -> f32 {
		const EPS: f32 = 1.0;
		let h = self.height_at(position);
		let hx = self.height_at(position + Vec3::new(EPS, 0.0, 0.0));
		let hz = self.height_at(position + Vec3::new(0.0, 0.0, EPS));
		let dx = (hx - h) / EPS;
		let dz = (hz - h) / EPS;
		(dx * dx + dz * dz).sqrt()
	}

	fn exclusion_zones(&self) -> &[bevy::math::bounding::Aabb3d] {
		self.base.exclusion_zones()
	}

	fn allows_placement_at(&self, position: Vec3) -> bool {
		if !self.base.allows_placement_at(position) {
			return false;
		}
		!matches!(
			self.pads.classification_at(position.x, position.z),
			Some(PadStage::Flatten | PadStage::Grade)
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::math::Vec2;
	use chico_groves::FlatTerrainSample;
	use richmond_development_models::PadParams;

	#[test]
	fn pad_modulation_sets_exact_terrace_and_preserves_base_outside() -> Result<()> {
		let pad = PadComplex::building_skirt(
			Vec2::ZERO,
			Vec2::splat(10.0),
			0.0,
			12.0,
			PadParams::default(),
		);
		let sample = DevelopmentGroveSample {
			base: FlatTerrainSample { elevation: 3.0, steepness: 0.0 },
			pads: pad,
		};

		assert!((sample.height_at(Vec3::ZERO) - 12.0).abs() < 1e-5);
		assert!((sample.height_at(Vec3::new(1_000.0, 0.0, 1_000.0)) - 3.0).abs() < 1e-5);
		assert!(!sample.allows_placement_at(Vec3::ZERO));
		assert!(sample.allows_placement_at(Vec3::new(1_000.0, 3.0, 1_000.0)));
		Ok(())
	}
}
