//! Reusable Richmond building scene components.
//!
//! Per domain: [`style`](floors::FloorStyle) + geometry + [`Placement`] → node (`LodScene`).

pub mod arc_kit;
pub mod assets;
pub mod doors;
pub mod floors;
pub mod furniture;
pub mod joints;
pub mod labels;
pub mod layer;
pub mod lod_band;
pub mod lod_host;
pub mod panels;
pub mod parent_confines;
pub mod partitions;
pub mod placed;
pub mod roofs;
pub mod scene_children;
pub mod stairs;
pub mod structural_probe;

pub use arc_kit::{arc_ring_dir, arc_ring_dir_deg, decompose_arc_sweep, ArcKit};
pub use assets::AssetPath;
pub use doors::DoorNode;
pub use floors::FloorNode;
pub use furniture::{FurnitureGeometry, FurnitureNode, FurnitureStyle, FurnitureWireframePlugin};
pub use joints::{JointGeometry, JointNode, JointStyle};
pub use labels::{LabelGeometry, LabelNode, LabelStyle, LabelWireframePlugin};
pub use layer::{Layer, Layers};
pub use lod_band::{warm_mesh_lod_culls, warm_mesh_lod_culls_at_depth};
pub use lod_host::{
	posed_asset_tier, warm_content_host, warm_content_host_hsl, warm_content_host_hslu,
	warm_mesh_level_host, WarmAssetLodRoots,
};
pub use panels::{
	dihedral_kink, fitted_tile_count, to_centered_rect_placement, triangle_normal,
	update_panel_host_levels, with_wall_standup_pitch, PanelGeometry, PanelKitCaps, PanelLodBand,
	PanelLodProbe, PanelNode, PanelStyle, Rectangle as PanelRectangle,
	RightTriangle as PanelRightTriangle, TessellatedTriangle, DEFAULT_MIN_JOINT_ANGLE,
	DEFAULT_TILE_WIDTH, PANEL_HIGH_FACTOR, PANEL_LOW_FACTOR, PANEL_MEDIUM_FACTOR,
	PANEL_ULTRA_LOW_RECTANGLE, PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
};
pub use parent_confines::{
	apply_parent_confines, confined_scene, distance_to_segment, InternalShape, ParentConfines,
	INTERNAL_REVEAL_FACTOR,
};
pub use partitions::{
	update_partition_host_levels, Partition, PartitionGeometry, PartitionLodBand,
	PartitionLodProbe, PartitionMeshSet, PartitionMeshTier, PartitionNode, PartitionStyle,
	LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR, SLICE_KIT_HEIGHT,
};
pub use placed::{Placed, Placement};
pub use roofs::{
	update_roof_host_levels, Pitch, RoofGeometry, RoofLodBand, RoofLodProbe, RoofNode, RoofStyle,
	ROOF_HIGH_FACTOR, ROOF_LOW_FACTOR, ROOF_MEDIUM_FACTOR,
};
pub use scene_children::{
	pose, posed_glb, posed_scene, scene_children, wireframe_box_with_handles, with_pose,
};
pub use stairs::StairNode;
pub use structural_probe::{
	distance_outside_aabb2d_xz, distance_outside_footprints,
	update_building_structural_host_levels, BuildingStructuralLodProbe,
	STRUCTURAL_HIGH_OUTSIDE_METERS,
};

use bevy::scene::{ResolveContext, ResolvedScene, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

/// Domain IR exposed by a building (or building part) for structural composition.
///
/// Each method returns nodes of one domain type, grouped by provenance [`Layer`]
/// (see [`Layers`]). Layer identity is **not** node-type identity—it records where
/// geometry came from so parents can apply policy. Prefer [`Layers::free`] until a
/// provenance name is meaningful. Buildings compose by [`Layers::extend`].
pub trait BuildingComponents {
	fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
		Layers::new()
	}

	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::new()
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}

	fn roof_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RoofNode> {
		Layers::new()
	}

	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::new()
	}

	fn door_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<DoorNode> {
		Layers::new()
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<JointNode> {
		Layers::new()
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::new()
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::new()
	}

	/// When set, [`ComponentsOnly`] presents a warm High/Medium host driven by this probe.
	fn structural_lod_probe(&self) -> Option<BuildingStructuralLodProbe> {
		None
	}
}

impl<T: BuildingComponents + ?Sized> BuildingComponents for &T {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		(**self).panel_nodes_for_level(level)
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		(**self).partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		(**self).floor_nodes_for_level(level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		(**self).roof_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		(**self).stair_nodes_for_level(level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		(**self).door_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		(**self).joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		(**self).furniture_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		(**self).label_nodes_for_level(level)
	}

	fn structural_lod_probe(&self) -> Option<BuildingStructuralLodProbe> {
		(**self).structural_lod_probe()
	}
}

/// Newtype: present a [`BuildingComponents`] value as an [`LodScene`] whose children are
/// exactly that building's domain nodes.
///
/// Prefer this over a custom `LodScene` when the building has no host banding, silhouette,
/// lights, or other non-node extras. Orphan rules prevent a blanket `LodScene` for all
/// `BuildingComponents` implementors; wrapping in this local type is the coherent path.
///
/// ```ignore
/// ComponentsOnly(&bedroom).scene_with_lod(lod_ref)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentsOnly<T>(pub T);

impl<T> ComponentsOnly<T> {
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T> From<T> for ComponentsOnly<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T> std::ops::Deref for ComponentsOnly<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<T> std::ops::DerefMut for ComponentsOnly<T> {
	fn deref_mut(&mut self) -> &mut T {
		&mut self.0
	}
}

impl<T: BuildingComponents> BuildingComponents for ComponentsOnly<T> {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.0.panel_nodes_for_level(level)
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		self.0.partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.0.floor_nodes_for_level(level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		self.0.roof_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.0.stair_nodes_for_level(level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		self.0.door_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.0.joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		self.0.furniture_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.0.label_nodes_for_level(level)
	}

	fn structural_lod_probe(&self) -> Option<BuildingStructuralLodProbe> {
		self.0.structural_lod_probe()
	}
}

impl<T: BuildingComponents> LodScene for ComponentsOnly<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.0
			.structural_lod_probe()
			.map(|p| p.level_for(lod_ref.current_transform))
			.unwrap_or(LodSceneLevel::High)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match self.0.structural_lod_probe() {
			Some(probe) => probe.status_for_lod_ref(lod_ref),
			None => LodSceneStatus::Unchanged,
		}
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		// Keep structural H/M/L roots warm; content differs per band and respawn is expensive.
		LodSceneCulls::None
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		component_only_scene(&self.0, lod_ref, level)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		match self.0.structural_lod_probe() {
			Some(probe) => {
				// Medium and Low share shell-only content for this banding policy.
				let mid = component_only_scene(&self.0, lod_ref, LodSceneLevel::Medium);
				Box::new(warm_content_host_hsl(
					level,
					probe,
					component_only_scene(&self.0, lod_ref, LodSceneLevel::High),
					mid,
					component_only_scene(&self.0, lod_ref, LodSceneLevel::Medium),
				)) as Box<dyn Scene>
			}
			None => Box::new(component_only_scene(&self.0, lod_ref, level)) as Box<dyn Scene>,
		}
	}
}

/// Append every domain node from `building` at `level` as nested [`LodScene`] children.
///
/// Provenance is flattened away ([`Layers::flatten`]) for presentation today; parents
/// that need layer policy should read [`BuildingComponents`] maps before this step.
pub fn append_component_scenes(
	building: &impl BuildingComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in building.panel_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.partition_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.floor_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.roof_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.stair_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.door_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.joint_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.furniture_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
	for node in building.label_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_lod(lod_ref)));
	}
}

/// Scene whose children are exactly the [`BuildingComponents`] nodes at `level`.
pub fn component_only_scene(
	building: &impl BuildingComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_component_scenes(building, lod_ref, level, &mut children);
	scene_children(children)
}

pub(crate) fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

/// Shared empty `LodScene` body for component placeholders.
macro_rules! impl_empty_lod_scene {
	($($ty:ty),+ $(,)?) => {
		$(
			impl ::lod::gen::LodScene for $ty {
				fn scene_lod_status(
					&self,
					_lod_ref: &::lod::lod_ref::LodRef,
				) -> ::lod::gen::LodSceneStatus {
					::lod::gen::LodSceneStatus::Unchanged
				}

				fn scene_with_level(
					&self,
					_lod_ref: &::lod::lod_ref::LodRef,
					_level: ::lod::gen::LodSceneLevel,
				) -> impl ::bevy::scene::Scene + 'static {
					::bevy::scene::SceneFunction($crate::empty_scene)
				}
			}
		)+
	};
}

pub(crate) use impl_empty_lod_scene;

/// `LodScene` that loads a GLB scene root via [`scene_ref::SceneRef`].
macro_rules! impl_glb_lod_scene {
	($ty:ty, $asset:expr) => {
		impl ::lod::gen::LodScene for $ty {
			fn scene_lod_status(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::gen::LodSceneStatus {
				::lod::gen::LodSceneStatus::Unchanged
			}

			fn scene_with_level(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				_level: ::lod::gen::LodSceneLevel,
			) -> impl ::bevy::scene::Scene + 'static {
				($asset).scene_ref().scene()
			}
		}
	};
}

pub(crate) use impl_glb_lod_scene;
