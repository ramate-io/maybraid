//! [`GenerationScheme`]s and the [`GenerationContext`] they generate through.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy::math::DVec3;
use bevy::shape::Aabb3d;

use crate::gen::{Id, OriginalId};

use super::node_store::DEFAULT_BASE_SCALE;
use super::storage::store::{ErasedStore, TypedStore};
use super::storage::{HcsgStorage, HcsgValue};

/// How one generated type is discovered and built.
///
/// A value is a pure function of its key `(TypeId, Id)` and the session roots.
/// Schemes read only through the [`GenerationContext`].
pub trait GenerationScheme: HcsgValue + Sized {
	/// Level-0 cell size of this type's spatial index ([`HcsgStorage`]).
	const INDEX_SCALE: DVec3 = DEFAULT_BASE_SCALE;

	/// Hysteresis pad around live subscription regions during eviction sweeps.
	const RETENTION_MARGIN: DVec3 = Self::INDEX_SCALE;

	/// Ids that originate in `region`. May generate dependencies through `cx`.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>;

	/// The value for `id` and its bounds, or `None` where nothing exists.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)>;
}

/// Storage access for schemes: owned `Arc` dependencies, generated on demand.
///
/// Locks are taken only to look up and publish, so a dependency stays usable
/// after its store is released, and recursion never holds a lock.
pub struct GenerationContext<'a> {
	storage: &'a HcsgStorage,
	/// Resolved typed stores for this fill, avoiding registry reads on every lookup.
	stores: HashMap<TypeId, Arc<dyn ErasedStore>>,
	/// `(TypeId, Id)` pairs being built on this stack; a repeat is a cycle.
	generating: HashSet<(TypeId, Id)>,
	/// Types this fill read, hit or miss. The worker unions this into the
	/// subscription's reach.
	reached: HashSet<TypeId>,
	/// How many `build_with_id` frames sit above the current generate.
	nested_depth: usize,
	stale: &'a (dyn Fn() -> bool + 'a),
	#[cfg(feature = "generation-tracing")]
	trace_operation: Option<lod_generation_diagnostics::Operation>,
	#[cfg(feature = "generation-tracing")]
	trace_depth: usize,
}

impl<'a> GenerationContext<'a> {
	pub fn new(storage: &'a HcsgStorage) -> Self {
		Self::with_origin(storage, "direct")
	}

	/// Label a synchronous caller without changing generation or cache semantics.
	pub fn with_origin(storage: &'a HcsgStorage, _origin: &'static str) -> Self {
		let context = Self::create(storage, &|| false);
		#[cfg(feature = "generation-tracing")]
		let context = {
			let mut context = context;
			context.trace_operation = lod_generation_diagnostics::Operation::new(
				_origin,
				if _origin == "journey-poi" { "WorldPoiTiles" } else { "direct" },
				Default::default(),
			);
			context
		};
		context
	}

	/// Once `stale` returns true, nothing more is generated or published.
	pub fn with_stale(storage: &'a HcsgStorage, stale: &'a (dyn Fn() -> bool + 'a)) -> Self {
		let context = Self::create(storage, stale);
		#[cfg(feature = "generation-tracing")]
		let context = {
			let mut context = context;
			context.trace_operation =
				lod_generation_diagnostics::Operation::new("direct", "direct", Default::default());
			context
		};
		context
	}

	#[cfg(feature = "generation-tracing")]
	pub(super) fn for_worker(
		storage: &'a HcsgStorage,
		stale: &'a (dyn Fn() -> bool + 'a),
		origin: &'static str,
		ty: &'static str,
		correlation: lod_generation_diagnostics::Correlation,
	) -> Self {
		let mut context = Self::create(storage, stale);
		context.trace_operation =
			lod_generation_diagnostics::Operation::new(origin, ty, correlation);
		context
	}

	fn create(storage: &'a HcsgStorage, stale: &'a (dyn Fn() -> bool + 'a)) -> Self {
		Self {
			storage,
			stores: HashMap::new(),
			generating: HashSet::new(),
			reached: HashSet::new(),
			nested_depth: 0,
			stale,
			#[cfg(feature = "generation-tracing")]
			trace_operation: None,
			#[cfg(feature = "generation-tracing")]
			trace_depth: 0,
		}
	}

	#[cfg(feature = "generation-tracing")]
	pub(super) fn trace_scope(&self) -> Option<lod_generation_diagnostics::EnteredOperation> {
		self.trace_operation
			.as_ref()
			.and_then(lod_generation_diagnostics::Operation::enter)
	}

	pub fn is_stale(&self) -> bool {
		(self.stale)()
	}

	fn touch<T: HcsgValue>(&mut self) {
		self.reached.insert(TypeId::of::<T>());
	}

	fn typed_store<T: HcsgValue>(&mut self) -> Option<Arc<TypedStore<T>>> {
		let type_id = TypeId::of::<T>();
		if let Some(store) = self.stores.get(&type_id) {
			return store.clone().into_any().downcast::<TypedStore<T>>().ok();
		}
		let store = self.storage.store::<T>()?;
		self.stores.insert(type_id, store.clone());
		Some(store)
	}

	fn stored<T: HcsgValue>(&mut self, id: Id) -> Option<Arc<T>> {
		let value = self.typed_store::<T>().and_then(|store| store.read().value(id).cloned());
		#[cfg(feature = "generation-tracing")]
		lod_generation_diagnostics::update(|c| {
			if value.is_some() {
				c.cache_hits += 1;
			} else {
				c.cache_misses += 1;
			}
		});
		value
	}

	/// Types this context read since it was created.
	pub(super) fn take_reached(&mut self) -> HashSet<TypeId> {
		std::mem::take(&mut self.reached)
	}

	/// Bounds of a value already stored for `id`, recording `T` on this fill's reach.
	pub fn stored_bounds<T: HcsgValue>(&mut self, id: Id) -> Option<Aabb3d> {
		#[cfg(feature = "generation-tracing")]
		let _scope = self.trace_scope();
		self.touch::<T>();
		let store = self.typed_store::<T>()?;
		let nodes = store.read();
		nodes.entry(id).map(|entry| entry.bounds)
	}

	pub fn get<T: HcsgValue>(&mut self, id: Id) -> Option<Arc<T>> {
		#[cfg(feature = "generation-tracing")]
		let _scope = self.trace_scope();
		self.touch::<T>();
		self.stored::<T>(id)
	}

	pub fn original_ids_for<T: GenerationScheme>(&mut self, region: Aabb3d) -> Vec<OriginalId> {
		#[cfg(feature = "generation-tracing")]
		let _scope = self.trace_scope();
		#[cfg(feature = "generation-tracing")]
		let _phase = lod_generation_diagnostics::Phase::new(
			"discover",
			std::any::type_name::<T>(),
			self.trace_depth,
		);
		#[cfg(feature = "generation-tracing")]
		{
			lod_generation_diagnostics::update(|c| c.discoveries += 1);
			self.trace_depth += 1;
		}
		let ids = T::original_ids_for(self, region);
		#[cfg(feature = "generation-tracing")]
		{
			self.trace_depth -= 1;
		}
		ids
	}

	/// The stored value, else builds and publishes it, resolving its own
	/// dependencies on the way. `None` where nothing exists, on a cycle, or
	/// once stale.
	pub fn get_or_generate<T: GenerationScheme>(&mut self, id: Id) -> Option<Arc<T>> {
		#[cfg(feature = "generation-tracing")]
		let _scope = self.trace_scope();
		self.touch::<T>();
		if let Some(value) = self.stored::<T>(id) {
			return Some(value);
		}
		self.generate_missing::<T>(id)
	}

	/// Worker path: one store lookup on cache hits, no redundant miss check.
	pub(in crate::hcsg) fn generate_cached<T: GenerationScheme>(&mut self, id: Id) -> bool {
		#[cfg(feature = "generation-tracing")]
		let _scope = self.trace_scope();
		self.touch::<T>();
		if self.stored::<T>(id).is_some() {
			return true;
		}
		self.generate_missing::<T>(id).is_some()
	}

	/// Builds and publishes `id` when it is not already stored.
	fn generate_missing<T: GenerationScheme>(&mut self, id: Id) -> Option<Arc<T>> {
		let key = (TypeId::of::<T>(), id);
		if self.is_stale() {
			#[cfg(feature = "generation-tracing")]
			lod_generation_diagnostics::update(|c| c.cancelled += 1);
			return None;
		}
		if !self.generating.insert(key) {
			#[cfg(feature = "generation-tracing")]
			lod_generation_diagnostics::update(|c| c.cycles += 1);
			return None;
		}
		if self.storage.take_evicted(TypeId::of::<T>(), id) {
			self.storage.record_rebuild(self.nested_depth > 0);
		}
		#[cfg(feature = "generation-tracing")]
		let phase = lod_generation_diagnostics::Phase::new(
			"build",
			std::any::type_name::<T>(),
			self.trace_depth,
		);
		#[cfg(feature = "generation-tracing")]
		{
			lod_generation_diagnostics::update(|c| c.builds += 1);
			self.trace_depth += 1;
		}
		self.nested_depth += 1;
		let built = T::build_with_id(self, id);
		#[cfg(feature = "generation-tracing")]
		{
			self.trace_depth -= 1;
			drop(phase);
			if built.is_none() {
				lod_generation_diagnostics::update(|c| c.absent += 1);
			}
		}
		self.nested_depth -= 1;
		self.generating.remove(&key);
		let (value, bounds) = built?;
		let value = Arc::new(value);
		self.storage.register_scheme::<T>();
		self.storage.publish_unless(id, Arc::clone(&value), bounds, self.stale)?;
		Some(value)
	}

	/// The stored value for `id` and its published bounds after generate.
	pub fn get_or_generate_stored<T: GenerationScheme>(
		&mut self,
		id: Id,
	) -> Option<(Arc<T>, Aabb3d)> {
		let value = self.get_or_generate::<T>(id)?;
		let bounds = self.stored_bounds::<T>(id)?;
		Some((value, bounds))
	}

	/// Every `T` originating in `region` that exists, in id order.
	pub fn get_or_generate_in<T: GenerationScheme>(&mut self, region: Aabb3d) -> Vec<Arc<T>> {
		#[cfg(feature = "generation-tracing")]
		let _scope = self.trace_scope();
		self.sorted_ids_for::<T>(region)
			.into_iter()
			.filter_map(|id| self.get_or_generate::<T>(id))
			.collect()
	}

	/// Like [`get_or_generate_in`], but sorted narrowest `footprint` first.
	///
	/// Overlapping cells at different resolutions sample correctly when the
	/// finest cell covering a point should win.
	pub fn get_or_generate_in_finest_first<T: GenerationScheme>(
		&mut self,
		region: Aabb3d,
		footprint: impl Fn(&T) -> Aabb3d,
	) -> Vec<Arc<T>> {
		let mut cells = self.get_or_generate_in::<T>(region);
		cells.sort_by(|a, b| aabb3_span_x(footprint(a)).total_cmp(&aabb3_span_x(footprint(b))));
		cells
	}

	/// Every `T` originating in `region`, in id order; `None` if one of them
	/// is missing, for values that can't compose a partial region.
	pub fn get_or_generate_all_in<T: GenerationScheme>(
		&mut self,
		region: Aabb3d,
	) -> Option<Vec<Arc<T>>> {
		self.sorted_ids_for::<T>(region)
			.into_iter()
			.map(|id| self.get_or_generate::<T>(id))
			.collect()
	}

	fn sorted_ids_for<T: GenerationScheme>(&mut self, region: Aabb3d) -> Vec<Id> {
		let mut ids: Vec<Id> = self
			.original_ids_for::<T>(region)
			.into_iter()
			.map(|OriginalId(id)| id)
			.collect();
		ids.sort();
		ids.dedup();
		ids
	}
}

fn aabb3_span_x(bounds: Aabb3d) -> f32 {
	bounds.max.x - bounds.min.x
}
