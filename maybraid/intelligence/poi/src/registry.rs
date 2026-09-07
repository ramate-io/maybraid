use std::collections::{BTreeMap, BTreeSet, HashMap};

use bevy::math::{bounding::Aabb3d, DVec3};
use bevy::prelude::*;
use gimme_core::{BaseScale, HashMapStore, Level, SpatialId, SpatialIndexError, TypedIndex};

use crate::hash::unit_f32;
use crate::{
	NearbyChoice, NearbyQuery, Poi, PoiId, PoiInterests, PoiKind, PoiObservation, PoiSource,
	MAX_POI_ARRIVAL_RADIUS,
};

const LOCAL_BASE_SCALE: f64 = 64.0;
const MAX_LOCAL_QUERY_RADIUS: f32 = 1_000.0;

/// Indexed world snapshot used by local and global discovery scans.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoiRecord {
	pub id: PoiId,
	pub entity: Entity,
	pub kind: PoiKind,
	pub position: Vec3,
	pub arrival_radius: f32,
	pub salience: f32,
	pub local: bool,
	pub global: bool,
}

impl PoiRecord {
	pub fn observation(self, user: Entity, source: PoiSource) -> PoiObservation {
		PoiObservation {
			user,
			id: self.id,
			entity: Some(self.entity),
			kind: self.kind,
			position: self.position,
			arrival_radius: self.arrival_radius,
			salience: self.salience,
			confidence: 1.0,
			source,
		}
	}
}

impl SpatialId for PoiRecord {
	type Id = PoiId;

	fn spatial_id(&self) -> Self::Id {
		self.id
	}
}

/// Gimme-backed local index plus a sparse global set.
#[derive(Resource)]
pub struct PoiRegistry {
	local: TypedIndex<PoiRecord, HashMapStore<PoiRecord>>,
	records: BTreeMap<PoiId, PoiRecord>,
	globals: BTreeSet<PoiId>,
	by_entity: HashMap<Entity, PoiId>,
	local_max_level: Level,
}

impl Default for PoiRegistry {
	fn default() -> Self {
		let scale = DVec3::splat(LOCAL_BASE_SCALE);
		let local = match TypedIndex::new(scale, HashMapStore::new()) {
			Ok(index) => index,
			Err(error) => unreachable!("fixed positive POI base scale was rejected: {error}"),
		};
		Self {
			local,
			records: BTreeMap::new(),
			globals: BTreeSet::new(),
			by_entity: HashMap::new(),
			local_max_level: 0,
		}
	}
}

impl PoiRegistry {
	pub fn upsert(
		&mut self,
		entity: Entity,
		poi: Poi,
		position: Vec3,
		local: bool,
		global: bool,
	) -> Result<(), SpatialIndexError> {
		if self.records.get(&poi.id).is_some_and(|previous| previous.entity != entity)
			|| !position.is_finite()
			|| !poi.arrival_radius.is_finite()
			|| !poi.salience.is_finite()
		{
			self.remove_entity(entity);
			return Err(SpatialIndexError::InsertFailed);
		}
		if let Some(previous_id) = self.by_entity.get(&entity).copied() {
			if previous_id != poi.id {
				self.remove_id(previous_id);
			}
		}
		self.by_entity.insert(entity, poi.id);

		self.local.remove_value(poi.id);
		self.globals.remove(&poi.id);
		let record = PoiRecord {
			id: poi.id,
			entity,
			kind: poi.kind,
			position,
			arrival_radius: poi.arrival_radius.clamp(0.0, MAX_POI_ARRIVAL_RADIUS),
			salience: poi.salience.max(0.0),
			local,
			global,
		};
		self.records.insert(poi.id, record);

		if local {
			let extent = Vec3::splat(record.arrival_radius.max(0.05));
			let bounds = Aabb3d::from_min_max(position - extent, position + extent);
			let base = BaseScale::new(self.local.grid().base_scale())?;
			self.local_max_level = self.local_max_level.max(base.insertion_level(&bounds));
			self.local.insert_value(record, bounds)?;
		}
		if global {
			self.globals.insert(poi.id);
		}
		Ok(())
	}

	pub fn remove_entity(&mut self, entity: Entity) -> Option<PoiRecord> {
		let id = self.by_entity.remove(&entity)?;
		self.remove_id(id)
	}

	pub fn get(&self, id: PoiId) -> Option<&PoiRecord> {
		self.records.get(&id)
	}

	pub fn local_matching(
		&self,
		center: Vec3,
		radius: f32,
		interests: &PoiInterests,
	) -> Vec<PoiRecord> {
		if !center.is_finite() || !radius.is_finite() {
			return Vec::new();
		}
		let radius = radius.clamp(0.0, MAX_LOCAL_QUERY_RADIUS);
		let extent = Vec3::splat(radius);
		let region = Aabb3d::from_min_max(center - extent, center + extent);
		let query_level = BaseScale::new(self.local.grid().base_scale())
			.map(|base| base.insertion_level(&region))
			.unwrap_or(0);
		self.local
			.query_values(region, BaseScale::levels_through(query_level.max(self.local_max_level)))
			.map(|(record, _)| *record)
			.filter(|record| {
				interests.contains(record.kind)
					&& center.distance(record.position) <= radius + record.arrival_radius
			})
			.collect()
	}

	pub fn global_matching(&self, interests: &PoiInterests) -> Vec<PoiRecord> {
		self.globals
			.iter()
			.filter_map(|id| self.records.get(id).copied())
			.filter(|record| interests.contains(record.kind))
			.collect()
	}

	/// Weighted nearby choice with no inner hole. See [`Self::choose_in`].
	pub fn choose_nearby(
		&self,
		center: Vec3,
		radius: f32,
		interests: &PoiInterests,
		excluded: &[PoiId],
		seed: u64,
	) -> Option<PoiRecord> {
		self.choose_in(center, NearbyQuery::weighted(radius), interests, excluded, seed)
	}

	/// Choose a local/global POI inside [`NearbyQuery`].
	///
	/// Candidates closer than `min_radius` on XZ are dropped. `excluded` IDs
	/// are dropped while another candidate remains. [`NearbyChoice::Weighted`]
	/// uses interest, salience, and proximity; [`NearbyChoice::Nearest`] takes
	/// the closest remaining XZ pose.
	pub fn choose_in(
		&self,
		center: Vec3,
		query: NearbyQuery,
		interests: &PoiInterests,
		excluded: &[PoiId],
		seed: u64,
	) -> Option<PoiRecord> {
		if interests.is_empty() || !center.is_finite() || !query.radius.is_finite() {
			return None;
		}
		let radius = query.radius.clamp(0.0, MAX_LOCAL_QUERY_RADIUS);
		let min_radius = query.min_radius.max(0.0);
		let mut candidates = self.local_matching(center, radius, interests);
		for candidate in self.global_matching(interests) {
			if center.distance(candidate.position) <= radius + candidate.arrival_radius
				&& !candidates.iter().any(|known| known.id == candidate.id)
			{
				candidates.push(candidate);
			}
		}
		candidates.retain(|candidate| xz_distance(center, candidate.position) >= min_radius);
		candidates.sort_by_key(|candidate| candidate.id);
		Self::drop_excluded(&mut candidates, excluded);
		match query.choice {
			NearbyChoice::Nearest => candidates.into_iter().min_by(|a, b| {
				xz_distance(center, a.position)
					.total_cmp(&xz_distance(center, b.position))
					.then_with(|| a.id.cmp(&b.id))
			}),
			NearbyChoice::Weighted => choose_weighted(center, radius, interests, candidates, seed),
		}
	}

	pub fn matching_in_xz_tile(
		&self,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
	) -> Vec<PoiRecord> {
		if !tile_size.is_finite() || !center_y.is_finite() {
			return Vec::new();
		}
		let size = tile_size.clamp(1.0, MAX_LOCAL_QUERY_RADIUS);
		let min_xz = tile.as_vec2() * size;
		let max_xz = min_xz + Vec2::splat(size);
		let region = Aabb3d::from_min_max(
			Vec3::new(min_xz.x, center_y - MAX_LOCAL_QUERY_RADIUS, min_xz.y),
			Vec3::new(max_xz.x, center_y + MAX_LOCAL_QUERY_RADIUS, max_xz.y),
		);
		let query_level = BaseScale::new(self.local.grid().base_scale())
			.map(|base| base.insertion_level(&region))
			.unwrap_or(0);
		let mut records = BTreeMap::new();
		for (record, _) in self
			.local
			.query_values(region, BaseScale::levels_through(query_level.max(self.local_max_level)))
		{
			if interests.contains(record.kind) && xz_tile(record.position, size) == tile {
				records.insert(record.id, *record);
			}
		}
		for record in self.global_matching(interests) {
			if xz_tile(record.position, size) == tile {
				records.insert(record.id, record);
			}
		}
		records.into_values().collect()
	}

	pub fn len(&self) -> usize {
		self.records.len()
	}

	pub fn is_empty(&self) -> bool {
		self.records.is_empty()
	}

	fn drop_excluded(candidates: &mut Vec<PoiRecord>, excluded: &[PoiId]) {
		for id in excluded {
			if candidates.len() <= 1 {
				return;
			}
			candidates.retain(|candidate| candidate.id != *id);
		}
	}

	fn remove_id(&mut self, id: PoiId) -> Option<PoiRecord> {
		self.local.remove_value(id);
		self.globals.remove(&id);
		let record = self.records.remove(&id)?;
		self.by_entity.remove(&record.entity);
		Some(record)
	}
}

fn choose_weighted(
	center: Vec3,
	radius: f32,
	interests: &PoiInterests,
	candidates: Vec<PoiRecord>,
	seed: u64,
) -> Option<PoiRecord> {
	let weight = |candidate: PoiRecord| {
		let interest = interests.weight(candidate.kind).unwrap_or(0.0);
		let proximity = 1.0 / (1.0 + center.distance(candidate.position) / radius.max(1.0));
		interest * candidate.salience.max(0.1) * proximity
	};
	let total: f32 = candidates.iter().copied().map(weight).sum();
	if total <= 0.0 {
		return None;
	}
	let mut draw = unit_f32(seed) * total;
	let mut fallback = None;
	for candidate in candidates {
		fallback = Some(candidate);
		draw -= weight(candidate);
		if draw <= 0.0 {
			return Some(candidate);
		}
	}
	fallback
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	(a.xz() - b.xz()).length()
}

fn xz_tile(position: Vec3, tile_size: f32) -> IVec2 {
	(position.xz() / tile_size).floor().as_ivec2()
}
