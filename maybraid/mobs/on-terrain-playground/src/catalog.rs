//! Short authored casts. World generation can put a dozen members in a cell;
//! this playground keeps the roster tiny so host Y and plant follow are readable.

use bevy::prelude::*;
use maybraid_mobs::{MobKind, MobScene};

/// Stable seed for `MobScene::of_kind`. Count is truncated after generation.
pub const PLAYGROUND_NUM: f32 = 4.0;
pub const HERD_MEMBERS: usize = 6;
pub const PACK_MEMBERS: usize = 4;
/// Whole 4×4 patch stays High so plants do not cull while the host journeys.
pub const HIGH_RADIUS: f32 = 2_000.0;

/// Which authored hosts to present.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaygroundCast {
	#[default]
	Herd,
	Pack,
	Both,
}

const HERD_PLACEMENT: [(MobKind, Vec2); 1] = [(MobKind::Herd, Vec2::new(-40.0, -20.0))];
const PACK_PLACEMENT: [(MobKind, Vec2); 1] = [(MobKind::Pack, Vec2::new(36.0, -16.0))];
const BOTH_PLACEMENTS: [(MobKind, Vec2); 2] =
	[(MobKind::Herd, Vec2::new(-40.0, -20.0)), (MobKind::Pack, Vec2::new(48.0, 24.0))];

impl PlaygroundCast {
	pub fn label(self) -> &'static str {
		match self {
			Self::Herd => "herd",
			Self::Pack => "pack",
			Self::Both => "herd+pack",
		}
	}

	/// `(kind, XZ offset from the terrain-patch center)`.
	pub fn placements(self) -> &'static [(MobKind, Vec2)] {
		match self {
			Self::Herd => &HERD_PLACEMENT,
			Self::Pack => &PACK_PLACEMENT,
			Self::Both => &BOTH_PLACEMENTS,
		}
	}
}

pub fn scene_for(kind: MobKind) -> MobScene {
	let mut scene = MobScene::of_kind(kind, PLAYGROUND_NUM).with_high_radius(HIGH_RADIUS);
	scene.mob.roster.members.truncate(member_cap(kind));
	scene
}

fn member_cap(kind: MobKind) -> usize {
	match kind {
		MobKind::Pack => PACK_MEMBERS,
		_ => HERD_MEMBERS,
	}
	.max(1)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn herd_and_pack_rosters_stay_short() {
		let herd = scene_for(MobKind::Herd);
		assert!(!herd.mob.roster.members.is_empty());
		assert!(herd.mob.roster.members.len() <= HERD_MEMBERS);
		assert_eq!(herd.mob.kind, MobKind::Herd);

		let pack = scene_for(MobKind::Pack);
		assert!(!pack.mob.roster.members.is_empty());
		assert!(pack.mob.roster.members.len() <= PACK_MEMBERS);
		assert_eq!(pack.mob.kind, MobKind::Pack);
	}

	#[test]
	fn both_places_a_herd_and_a_pack() {
		let kinds: Vec<_> =
			PlaygroundCast::Both.placements().iter().map(|(kind, _)| *kind).collect();
		assert_eq!(kinds, vec![MobKind::Herd, MobKind::Pack]);
	}
}
