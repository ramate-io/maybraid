//! Ground facts layers above terrain read: streaming, extent, collider order.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use bevy::ecs::schedule::SystemSet;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;

/// Whether fill and dependent streams may advance for model `M`.
#[derive(Resource, Debug, PartialEq, Eq)]
pub struct TerrainStreaming<M: Send + Sync + 'static> {
	pub enabled: bool,
	_m: PhantomData<fn() -> M>,
}

impl<M: Send + Sync + 'static> Clone for TerrainStreaming<M> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<M: Send + Sync + 'static> Copy for TerrainStreaming<M> {}

impl<M: Send + Sync + 'static> Default for TerrainStreaming<M> {
	fn default() -> Self {
		Self { enabled: true, _m: PhantomData }
	}
}

impl<M: Send + Sync + 'static> TerrainStreaming<M> {
	pub fn new(enabled: bool) -> Self {
		Self { enabled, _m: PhantomData }
	}
}

/// Run condition: [`TerrainStreaming<M>`] is on.
pub fn terrain_streaming<M: Send + Sync + 'static>(flag: Res<TerrainStreaming<M>>) -> bool {
	flag.enabled
}

/// Streamed ring or pinned patch. [`TerrainExtent<M>`] carries this for model `M`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerrainExtentKind {
	Streamed { presentation_region: Aabb3d },
	Pinned { region: Aabb3d },
}

/// Presentation footprint for model `M`: a moving stream or a pinned patch.
#[derive(Resource, Debug, PartialEq)]
pub struct TerrainExtent<M: Send + Sync + 'static> {
	pub kind: TerrainExtentKind,
	_m: PhantomData<fn() -> M>,
}

impl<M: Send + Sync + 'static> TerrainExtent<M> {
	pub fn streamed(presentation_region: Aabb3d) -> Self {
		Self { kind: TerrainExtentKind::Streamed { presentation_region }, _m: PhantomData }
	}

	pub fn pinned(region: Aabb3d) -> Self {
		Self { kind: TerrainExtentKind::Pinned { region }, _m: PhantomData }
	}

	pub fn is_streamed(&self) -> bool {
		matches!(self.kind, TerrainExtentKind::Streamed { .. })
	}

	pub fn presentation_region(&self) -> Aabb3d {
		match self.kind {
			TerrainExtentKind::Streamed { presentation_region } | TerrainExtentKind::Pinned { region: presentation_region } => {
				presentation_region
			}
		}
	}

	pub fn remap<N: Send + Sync + 'static>(&self) -> TerrainExtent<N> {
		TerrainExtent { kind: self.kind, _m: PhantomData }
	}
}

impl<M: Send + Sync + 'static> Clone for TerrainExtent<M> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<M: Send + Sync + 'static> Copy for TerrainExtent<M> {}

impl<M: Send + Sync + 'static> Default for TerrainExtent<M> {
	fn default() -> Self {
		Self::pinned(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO))
	}
}

/// Inner (`T` → `OnTerrain<T>`) then Outer (`M` → `Urbanization<M>`).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainContractForward {
	Inner,
	Outer,
}

/// Ordering sets layers use instead of Durham's collider enum.
pub struct TerrainLayerSystems<M: Send + Sync + 'static>(PhantomData<fn() -> M>);

impl<M: Send + Sync + 'static> TerrainLayerSystems<M> {
	#[allow(non_upper_case_globals)]
	pub const QueueColliders: Self = Self(PhantomData);
}

impl<M: Send + Sync + 'static> Clone for TerrainLayerSystems<M> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<M: Send + Sync + 'static> Copy for TerrainLayerSystems<M> {}

impl<M: Send + Sync + 'static> PartialEq for TerrainLayerSystems<M> {
	fn eq(&self, _other: &Self) -> bool {
		true
	}
}

impl<M: Send + Sync + 'static> Eq for TerrainLayerSystems<M> {}

impl<M: Send + Sync + 'static> Hash for TerrainLayerSystems<M> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		TypeId::of::<M>().hash(state);
	}
}

impl<M: Send + Sync + 'static> Debug for TerrainLayerSystems<M> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "TerrainLayerSystems<{}>::QueueColliders", std::any::type_name::<M>())
	}
}

impl<M: Send + Sync + 'static> SystemSet for TerrainLayerSystems<M> {
	fn dyn_clone(&self) -> Box<dyn SystemSet> {
		Box::new(*self)
	}
}

/// Copy streaming and extent from `From` onto wrapper `To`.
pub fn forward_terrain_contract<From, To>(
	from_streaming: Res<TerrainStreaming<From>>,
	mut to_streaming: ResMut<TerrainStreaming<To>>,
	from_extent: Res<TerrainExtent<From>>,
	mut to_extent: ResMut<TerrainExtent<To>>,
) where
	From: Send + Sync + 'static,
	To: Send + Sync + 'static,
{
	if from_streaming.is_changed() {
		to_streaming.enabled = from_streaming.enabled;
	}
	if from_extent.is_changed() {
		*to_extent = from_extent.remap();
	}
}

/// Install `To`'s contract resources and keep them equal to `From`.
pub fn install_terrain_contract_forward<From, To>(app: &mut App, set: TerrainContractForward)
where
	From: Send + Sync + 'static,
	To: Send + Sync + 'static,
{
	app.init_resource::<TerrainStreaming<To>>()
		.init_resource::<TerrainExtent<To>>()
		.add_systems(Update, forward_terrain_contract::<From, To>.in_set(set));
	app.add_systems(
		Update,
		queue_colliders_anchor::<To>
			.in_set(TerrainLayerSystems::<To>::QueueColliders)
			.before(TerrainLayerSystems::<From>::QueueColliders),
	);
}

fn queue_colliders_anchor<M: Send + Sync + 'static>() {}
