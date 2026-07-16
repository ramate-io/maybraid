use bevy::math::bounding::Aabb3d;

/// Spatial cell bounds for procedural origin ids.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell(pub Aabb3d);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginCell(pub Cell);

impl Eq for Cell {}

impl core::hash::Hash for Cell {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		self.0.min.x.to_bits().hash(state);
		self.0.min.y.to_bits().hash(state);
		self.0.min.z.to_bits().hash(state);
		self.0.max.x.to_bits().hash(state);
		self.0.max.y.to_bits().hash(state);
		self.0.max.z.to_bits().hash(state);
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bytes(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
	/// Some entities have a custom ID.
	Bytes(Bytes),

	/// Some entities, particularly procedural ones, are identified by their
	/// origin cell.
	OriginCell(OriginCell),
	/// The universal identifier, used for types that do not vary.
	Universal,
}

impl Id {
	pub fn from_cell(bounds: Aabb3d) -> Self {
		Self::OriginCell(OriginCell(Cell(bounds)))
	}

	pub fn origin_cell_bounds(self) -> Option<Aabb3d> {
		match self {
			Self::OriginCell(OriginCell(Cell(bounds))) => Some(bounds),
			Self::Bytes(_) => None,
			Self::Universal => None,
		}
	}
}

/// Ids that originate in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginalId(pub Id);

impl OriginalId {
	pub fn new(id: Id) -> Self {
		Self(id)
	}

	pub fn universal() -> Self {
		Self(Id::Universal)
	}
}

/// Ids that are tracked in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrackedId(pub Id);

/// Whether or not a given id is tracked in the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageStatus {
	NotTracked,
	TrackedWithin,
	TrackedOutside,
}
