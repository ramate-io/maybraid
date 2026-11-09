//! Typed posed plants shared by ordinary lazy presentation and resident packing.
use lod::{LodRef, LodSceneLevel, SceneChunk};
use material_ref::MaterialRef;
use std::sync::Arc;
use vegetation_components::{PlacedVegetation, Placement, VegetationComponents};

pub trait PosedPlantVisitor {
	type Output;
	fn plant<T: VegetationComponents + Send + Sync + 'static>(
		&mut self,
		plant: Arc<T>,
		placement: Placement,
		stick: &MaterialRef,
		ball: &MaterialRef,
		frond: &MaterialRef,
	) -> Self::Output;
}

pub trait PosedGrovePlant {
	fn visit<V: PosedPlantVisitor>(&self, visitor: &mut V) -> V::Output;
}

/// Ordinary presentation still captures its lazy list and original driver pose.
pub fn nest_posed_plant<P: PosedGrovePlant>(
	plant: &P,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	struct Nest<'a, 'b> {
		lod_ref: &'a LodRef<'b>,
		level: LodSceneLevel,
	}
	impl PosedPlantVisitor for Nest<'_, '_> {
		type Output = SceneChunk;
		fn plant<T: VegetationComponents + Send + Sync + 'static>(
			&mut self,
			plant: Arc<T>,
			placement: Placement,
			stick: &MaterialRef,
			ball: &MaterialRef,
			frond: &MaterialRef,
		) -> SceneChunk {
			super::nest_flattened_plant_chunk(
				plant,
				placement,
				stick,
				ball,
				frond,
				self.lod_ref,
				self.level,
			)
		}
	}
	plant.visit(&mut Nest { lod_ref, level })
}

pub fn placed<T: VegetationComponents + Send + Sync + 'static>(
	plant: Arc<T>,
	placement: Placement,
	stick: &MaterialRef,
	ball: &MaterialRef,
	frond: &MaterialRef,
) -> PlacedVegetation<Arc<T>> {
	PlacedVegetation::new(plant, placement, stick.clone(), ball.clone(), frond.clone())
}
