//! Per-jersey-cell modulation bundles used by [`crate::terrain::Terrain`].
//!
//! This is a data container only — Terrain pulls family layers and builds these
//! bundles itself (there is no separate compose [`lod::gen::GenerationScheme`]).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use jersey_terrain_stamps::JerseyModulation;

/// Per-family modulation counts for one jersey stamp cell (debug / inspection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JerseyFamilySummary {
	pub name: &'static str,
	pub modulation_count: usize,
}

/// Flattened jersey height ops for one jersey stamp cell (all families coexist).
#[derive(Debug, Clone, Component)]
pub struct JerseyModulations {
	pub cell: Aabb3d,
	pub modulations: Vec<JerseyModulation>,
	/// Which stamp families contributed, and how many ops each emitted.
	pub families: Vec<JerseyFamilySummary>,
}

impl JerseyModulations {
	pub(crate) fn append_layer(
		&mut self,
		name: &'static str,
		src: &[JerseyModulation],
	) {
		self.families.push(JerseyFamilySummary {
			name,
			modulation_count: src.len(),
		});
		self.modulations.extend(src.iter().cloned());
	}

	/// Compact label like `V3 P2 M1 C4 W2 R3` (family initial + op count).
	pub fn family_label(&self) -> String {
		self.families
			.iter()
			.map(|f| {
				let initial = f.name.chars().next().unwrap_or('?');
				format!("{initial}{}", f.modulation_count)
			})
			.collect::<Vec<_>>()
			.join(" ")
	}
}
