/// Default scan used by mob and world-player death replacement.
pub const DEFAULT_NEARBY_RADIUS: f32 = 160.0;

/// Horizontal ring around `center` when no nearby POI is available.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearbyFallback {
	pub min_radius: f32,
	pub max_radius: f32,
}

impl NearbyFallback {
	pub const fn new(min_radius: f32, max_radius: f32) -> Self {
		Self { min_radius, max_radius }
	}
}

/// How [`crate::place_nearby`] / [`crate::PoiRegistry::choose_in`] pick among POIs in range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NearbyChoice {
	/// Interest × salience × proximity, with last-id circulation.
	#[default]
	Weighted,
	/// Nearest XZ POI still outside [`NearbyQuery::min_radius`].
	Nearest,
}

/// Scan window for a nearby replacement pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearbyQuery {
	pub radius: f32,
	pub min_radius: f32,
	pub choice: NearbyChoice,
}

impl NearbyQuery {
	pub const fn weighted(radius: f32) -> Self {
		Self { radius, min_radius: 0.0, choice: NearbyChoice::Weighted }
	}

	pub const fn nearest_beyond(radius: f32, min_radius: f32) -> Self {
		Self { radius, min_radius, choice: NearbyChoice::Nearest }
	}
}
