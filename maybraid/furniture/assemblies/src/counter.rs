//! Counter: footer, inset volume, over-sailing top.

use crate::palette::wood;
use crate::Assembly;
use furniture_components::{slab, PartKind, PlacedPart};
use richmond_building_components::FurnitureGeometry;

/// Finish-only knobs. Topology does not change with [`Self::finish_seed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CounterParams {
	pub finish_seed: u64,
}

impl CounterParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Counter {
		Counter::from_params(*self)
	}
}

/// Toekick (footer) is the narrowest plan; volume is the cabinet; top over-sails.
#[derive(Clone, Debug, PartialEq)]
pub struct Counter {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Counter {
	pub fn from_params(params: CounterParams) -> Self {
		let seed = params.finish_seed;
		Self {
			finish_seed: seed,
			parts: vec![
				PlacedPart {
					kind: PartKind::CounterFooter,
					placement: slab(0.78, 0.0, 0.12),
					material: wood(seed, 1),
				},
				PlacedPart {
					kind: PartKind::CounterVolume,
					placement: slab(0.96, 0.12, 0.88),
					material: wood(seed, 2),
				},
				PlacedPart {
					kind: PartKind::CounterTop,
					placement: slab(1.04, 0.88, 1.0),
					material: wood(seed, 3),
				},
			],
		}
	}

	pub fn unit_from_num(num: u32) -> Self {
		CounterParams::unit_from_num(num).build()
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Counter,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn xz(part: &PlacedPart) -> f32 {
		part.placement.scale.x.min(part.placement.scale.z)
	}

	#[test]
	fn toekick_is_narrower_than_the_volume() -> anyhow::Result<()> {
		let counter = CounterParams::unit_from_num(8).build();
		let footer = counter
			.parts
			.iter()
			.find(|p| p.kind == PartKind::CounterFooter)
			.ok_or_else(|| anyhow::anyhow!("missing footer"))?;
		let volume = counter
			.parts
			.iter()
			.find(|p| p.kind == PartKind::CounterVolume)
			.ok_or_else(|| anyhow::anyhow!("missing volume"))?;
		let top = counter
			.parts
			.iter()
			.find(|p| p.kind == PartKind::CounterTop)
			.ok_or_else(|| anyhow::anyhow!("missing top"))?;
		if xz(footer) >= xz(volume) {
			return Err(anyhow::anyhow!("toekick must be narrower than the volume"));
		}
		if xz(top) <= xz(volume) {
			return Err(anyhow::anyhow!("countertop must oversail the volume"));
		}
		Ok(())
	}
}
