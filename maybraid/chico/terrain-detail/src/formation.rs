//! Formation identity — the forest-layer analog.
//!
//! Even 25% four-way throw at the 400 m tile center, then a 40 m
//! outcropping-or-`None` throw inside the winning formation. Every formation
//! can throw [`OutcroppingKind::SparseMix`] so the landscape is never rockless.

use bevy_math::Vec3;
use procedural_common::{BucketThrow, NoiseConfig, NoiseParams};

use crate::{FormationExtent, OutcroppingExtent, OutcroppingKind};

/// Keep the world origin off OpenSimplex's zero (same idea as forest hopscotch).
const SAMPLE_ORIGIN: Vec3 = Vec3::new(10_007.0, 0.0, 10_009.0);
const FORMATION_LANE: Vec3 = Vec3::new(61.0, 0.0, 0.0);
const OUTCROP_LANE: Vec3 = Vec3::new(67.0, 0.0, 0.0);

/// 400 m hopscotch identity. `Empty` is real — a sparse tile, not a void.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormationKind {
	BoulderField,
	CragComplex,
	MixedRocks,
	Empty,
}

impl FormationKind {
	pub const ALL: [Self; 4] =
		[Self::BoulderField, Self::CragComplex, Self::MixedRocks, Self::Empty];

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::BoulderField => "boulder-field",
			Self::CragComplex => "crag-complex",
			Self::MixedRocks => "mixed-rocks",
			Self::Empty => "empty",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		let key = name.trim().to_ascii_lowercase();
		Self::ALL.iter().copied().find(|kind| kind.as_kebab() == key)
	}

	/// Even 25% four-way throw at the formation-tile center.
	pub fn throw_on(extent: FormationExtent, noise: NoiseParams) -> Self {
		Self::throw_at(extent.center(), noise)
	}

	pub fn throw_at(position: Vec3, noise: NoiseParams) -> Self {
		let buckets = [Self::BoulderField, Self::CragComplex, Self::MixedRocks, Self::Empty];
		let throw = BucketThrow::from_weights(buckets.iter().map(|_| 1.0), 0.0);
		let n = NoiseConfig::new(noise);
		let sample = n.sample_3d(position + FORMATION_LANE + SAMPLE_ORIGIN) * throw.total_weight();
		let index = throw.select(sample).unwrap_or(3);
		buckets[index.min(buckets.len() - 1)]
	}

	/// 40 m outcropping-or-`None` throw inside this formation.
	pub fn throw_outcropping(
		self,
		cell: OutcroppingExtent,
		noise: NoiseParams,
	) -> Option<OutcroppingKind> {
		let buckets = self.outcropping_buckets();
		if buckets.is_empty() {
			return None;
		}
		let throw = BucketThrow::from_weights(buckets.iter().map(|(_, w)| *w), 0.0);
		let n = NoiseConfig::new(noise);
		let sample =
			n.sample_3d(cell.center() + OUTCROP_LANE + SAMPLE_ORIGIN) * throw.total_weight();
		let index = throw.select(sample)?;
		buckets.get(index).and_then(|(kind, _)| *kind)
	}

	fn outcropping_buckets(self) -> Vec<(Option<OutcroppingKind>, f32)> {
		match self {
			Self::BoulderField => vec![
				(Some(OutcroppingKind::SparseMix), 40.0),
				(None, 30.0),
				(Some(OutcroppingKind::BoulderPatch), 30.0),
			],
			Self::CragComplex => vec![
				(Some(OutcroppingKind::SparseMix), 30.0),
				(None, 30.0),
				(Some(OutcroppingKind::Crag), 40.0),
			],
			Self::MixedRocks => vec![
				(Some(OutcroppingKind::SparseMix), 40.0),
				(None, 40.0),
				(Some(OutcroppingKind::Loner), 5.0),
				(Some(OutcroppingKind::RockPile), 5.0),
				(Some(OutcroppingKind::BoulderPatch), 5.0),
				(Some(OutcroppingKind::Crag), 5.0),
			],
			Self::Empty => vec![(Some(OutcroppingKind::SparseMix), 50.0), (None, 50.0)],
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn kebab_round_trips_every_formation() -> Result<()> {
		for kind in FormationKind::ALL {
			assert_eq!(FormationKind::from_kebab(kind.as_kebab()), Some(kind));
		}
		assert!(FormationKind::from_kebab("not-a-formation").is_none());
		Ok(())
	}

	#[test]
	fn formation_throw_is_deterministic() -> Result<()> {
		let extent = FormationExtent::from_cell_index(0, 0);
		let noise = NoiseParams {
			seed: 1337,
			frequency: 0.0005,
			amplitude: 1.0,
			octaves: 1,
			..Default::default()
		};
		assert_eq!(FormationKind::throw_on(extent, noise), FormationKind::throw_on(extent, noise));
		Ok(())
	}

	#[test]
	fn empty_throws_sparse_mix_or_none() -> Result<()> {
		let noise = NoiseParams {
			seed: 3,
			frequency: 0.02,
			amplitude: 1.0,
			octaves: 1,
			..Default::default()
		};
		let mut saw_sparse = false;
		let mut saw_none = false;
		for iz in 0..12 {
			for ix in 0..12 {
				match FormationKind::Empty
					.throw_outcropping(OutcroppingExtent::from_cell_index(ix, iz), noise)
				{
					None => saw_none = true,
					Some(OutcroppingKind::SparseMix) => saw_sparse = true,
					Some(other) => anyhow::bail!("unexpected {other:?}"),
				}
			}
		}
		assert!(saw_sparse && saw_none);
		Ok(())
	}

	#[test]
	fn boulder_field_throws_sparse_mix_patch_or_none() -> Result<()> {
		let noise = NoiseParams {
			seed: 4,
			frequency: 0.02,
			amplitude: 1.0,
			octaves: 1,
			..Default::default()
		};
		let mut saw_patch = false;
		let mut saw_sparse = false;
		let mut saw_none = false;
		for iz in 0..12 {
			for ix in 0..12 {
				match FormationKind::BoulderField
					.throw_outcropping(OutcroppingExtent::from_cell_index(ix, iz), noise)
				{
					None => saw_none = true,
					Some(OutcroppingKind::BoulderPatch) => saw_patch = true,
					Some(OutcroppingKind::SparseMix) => saw_sparse = true,
					Some(other) => anyhow::bail!("unexpected {other:?}"),
				}
			}
		}
		assert!(saw_patch && saw_sparse && saw_none);
		Ok(())
	}

	#[test]
	fn mixed_rocks_can_produce_loners_and_piles() -> Result<()> {
		let noise = NoiseParams {
			seed: 11,
			frequency: 0.03,
			amplitude: 1.0,
			octaves: 1,
			..Default::default()
		};
		let mut loner = false;
		let mut pile = false;
		for iz in 0..20 {
			for ix in 0..20 {
				match FormationKind::MixedRocks
					.throw_outcropping(OutcroppingExtent::from_cell_index(ix, iz), noise)
				{
					Some(OutcroppingKind::Loner) => loner = true,
					Some(OutcroppingKind::RockPile) => pile = true,
					_ => {}
				}
			}
		}
		assert!(loner, "mixed rocks should throw Loner");
		assert!(pile, "mixed rocks should throw RockPile");
		Ok(())
	}

	#[test]
	fn crag_complex_rest_is_crag() -> Result<()> {
		let noise = NoiseParams {
			seed: 2,
			frequency: 0.02,
			amplitude: 1.0,
			octaves: 1,
			..Default::default()
		};
		let mut saw_crag = false;
		let mut saw_sparse = false;
		for iz in 0..12 {
			for ix in 0..12 {
				match FormationKind::CragComplex
					.throw_outcropping(OutcroppingExtent::from_cell_index(ix, iz), noise)
				{
					None => {}
					Some(OutcroppingKind::Crag) => saw_crag = true,
					Some(OutcroppingKind::SparseMix) => saw_sparse = true,
					Some(other) => anyhow::bail!("unexpected {other:?}"),
				}
			}
		}
		assert!(saw_crag && saw_sparse);
		Ok(())
	}
}
