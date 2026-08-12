//! Posed + material-stamped vegetation for nesting under grove [`LodScene`](lod::LodScene) hosts.

use bevy::prelude::Component;
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;

use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::node::FoliageNode;
use crate::layer::Layers;
use crate::placed::Placement;
use crate::sticks::node::StickNode;
use crate::structural_lod::StructuralLod;
use crate::VegetationComponents;

fn is_frond_geometry(geometry: &FoliageGeometry) -> bool {
	matches!(
		geometry,
		FoliageGeometry::FrondCollection(_)
			| FoliageGeometry::StraightFrond
			| FoliageGeometry::StraightFrondSegment
	)
}

/// Tree (or plant part) posed in parent space with grove palette materials.
///
/// Placement is baked into emitted nodes and [`structural_lod`] so banding uses
/// parent-local positions (camera [`LodRef`](lod::LodRef) is world-space). Nest as
/// [`crate::ComponentsOnly`]`<PlacedVegetation<T>>` under a grove host.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct PlacedVegetation<T: Send + Sync + 'static> {
	pub vegetation: T,
	pub placement: Placement,
	pub stick_material: MaterialRef,
	pub ball_material: MaterialRef,
	pub frond_material: MaterialRef,
}

impl<T: Send + Sync + 'static> PlacedVegetation<T> {
	pub fn new(
		vegetation: T,
		placement: Placement,
		stick_material: MaterialRef,
		ball_material: MaterialRef,
		frond_material: MaterialRef,
	) -> Self {
		Self {
			vegetation,
			placement,
			stick_material,
			ball_material,
			frond_material,
		}
	}
}

impl<T: VegetationComponents + Send + Sync + 'static> VegetationComponents for PlacedVegetation<T> {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = self
			.vegetation
			.stick_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(|mut node| {
				node.placement = self.placement.compose_child(node.placement);
				node.with_material(self.stick_material.clone())
			})
			.collect::<Vec<_>>();
		Layers::from_free(nodes)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let nodes = self
			.vegetation
			.foliage_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(|mut node| {
				node.placement = self.placement.compose_child(node.placement);
				let material = if is_frond_geometry(&node.geometry) {
					self.frond_material.clone()
				} else {
					self.ball_material.clone()
				};
				node.with_material(material)
			})
			.collect::<Vec<_>>();
		Layers::from_free(nodes)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		let lod = self.vegetation.structural_lod()?;
		let scale = self.placement.scale.abs().max_element().max(1e-4);
		let center = self
			.placement
			.compose_child(Placement::new(lod.center, 0.0))
			.translation;
		Some(
			StructuralLod::new(center, (lod.tree_radius * scale).max(1e-4))
				.with_factors(lod.high_factor, lod.medium_factor, lod.low_factor)
				.with_preserve_ultra_low(lod.preserve_ultra_low),
		)
	}
}
