//! Shared tuft-grove [`LodScene`]. Authored bands stay on the grove body.
//!
//! Tuft High / Medium / Low / UltraLow already live on [`VegetationComponents`].
//! This impl only makes the grove a structural host (same emission as the old
//! [`chico_vegetation_components::ComponentsOnly`] wrapper).

/// Mechanical [`LodScene`] for a tuft grove that already implements [`VegetationComponents`].
#[macro_export]
macro_rules! impl_tuft_grove_lod {
	($Grove:ty) => {
		impl lod::gen::LodScene for $Grove {
			fn scene_lod_level(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::gen::LodSceneLevel {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_level(band, lod_ref))
					.unwrap_or(lod::gen::LodSceneLevel::High)
			}

			fn scene_lod_status(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::gen::LodSceneStatus {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_status(band, lod_ref))
					.unwrap_or(lod::gen::LodSceneStatus::Unchanged)
			}

			fn scene_lod_culls(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				_current: lod::gen::LodSceneLevel,
			) -> lod::gen::LodSceneCulls {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_culls(band, lod_ref))
					.unwrap_or(lod::gen::LodSceneCulls::None)
			}

			fn scene_with_level(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				level: lod::gen::LodSceneLevel,
			) -> impl bevy::scene::prelude::Scene + 'static {
				chico_vegetation_components::component_only_scene(self, lod_ref, level)
			}

			fn scene_chunks_with_level(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				level: lod::gen::LodSceneLevel,
			) -> lod::SceneChunk {
				chico_vegetation_components::vegetation_scene_chunks(self, lod_ref, level)
			}

			fn scene_bounds(&self) -> bevy::math::bounding::Aabb3d {
				self.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| chico_vegetation_components::vegetation_bounds(self))
			}

			fn scene_with_lod(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
			) -> impl bevy::scene::prelude::Scene + 'static {
				lod::lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
			}
		}
	};
}
