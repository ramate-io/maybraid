//! Stack buffer for terrain-pitch column-walk skip lists.
//!
//! Terrain pitch excludes up to 32 ancestors per visual; each downward probe
//! can skip up to [`MAX_COLUMN_HITS`] additional Fixed volumes on the same
//! plumb line.

use bevy::ecs::entity::Entity;

/// Matches `character_motion::ancestor_exclude` walk cap.
pub const MAX_ANCESTOR_EXCLUDE: usize = 32;
/// Grove tiles and overlapping High-band plants on one plumb line.
pub const MAX_COLUMN_HITS: usize = 8;
pub const COLUMN_SKIP_CAP: usize = MAX_ANCESTOR_EXCLUDE + MAX_COLUMN_HITS;

/// Fixed-capacity skip list for one downward column probe.
#[derive(Clone, Copy, Debug)]
pub struct ColumnSkipBuffer {
	entities: [Entity; COLUMN_SKIP_CAP],
	len: usize,
}

impl ColumnSkipBuffer {
	pub fn from_exclude(exclude: &[Entity]) -> Self {
		debug_assert!(exclude.len() <= MAX_ANCESTOR_EXCLUDE);
		let mut buffer = Self::empty();
		for &entity in exclude {
			buffer.push(entity);
		}
		buffer
	}

	pub fn empty() -> Self {
		Self { entities: [Entity::PLACEHOLDER; COLUMN_SKIP_CAP], len: 0 }
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn push(&mut self, entity: Entity) -> bool {
		if self.len >= COLUMN_SKIP_CAP {
			return false;
		}
		self.entities[self.len] = entity;
		self.len += 1;
		true
	}

	pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
		self.entities[..self.len].iter().copied()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::entity::Entity;

	fn sample_entities(count: usize) -> Vec<Entity> {
		(0..count)
			.map(|index| Entity::from_raw_u32(index as u32 + 1).expect("test entity"))
			.collect()
	}

	#[test]
	fn from_exclude_copies_ancestors() {
		let exclude = sample_entities(4);
		let buffer = ColumnSkipBuffer::from_exclude(&exclude);
		assert_eq!(buffer.len(), 4);
		assert_eq!(buffer.iter().collect::<Vec<_>>(), exclude);
	}

	#[test]
	fn push_column_hits_until_cap() {
		let exclude = sample_entities(4);
		let mut buffer = ColumnSkipBuffer::from_exclude(&exclude);
		let hits = sample_entities(3);
		for entity in hits {
			assert!(buffer.push(entity));
		}
		assert_eq!(buffer.len(), 7);
	}

	#[test]
	fn push_stops_at_cap() {
		let mut buffer = ColumnSkipBuffer::from_exclude(&sample_entities(MAX_ANCESTOR_EXCLUDE));
		for index in 0..MAX_COLUMN_HITS {
			assert!(buffer.push(Entity::from_raw_u32(100 + index as u32).expect("hit")));
		}
		assert_eq!(buffer.len(), COLUMN_SKIP_CAP);
		assert!(!buffer.push(Entity::from_raw_u32(200).expect("overflow")));
	}
}
