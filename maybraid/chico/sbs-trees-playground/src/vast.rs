//! Tiled grove plains for LOD / scale testing.

use bevy::prelude::*;
use chico_groves::{
	AlpineParams, AridConiferSaplingParams, BraidGrassParams, BushScrubParams,
	ChristmasTaigaParams, CommonTuftsParams, ConiferMassivesParams, ConiferSaplingParams,
	DateGroveParams, DrylandParams, ForlornSavannaParams, GoettingenFollowParams, GroveExtent,
	HighBushParams, JerrysChaparralParams, JungleLowerMassivesParams, JungleMassivesParams,
	LeewardParams, LevantineScrubParams, LowBushParams, MonsterGrassParams, OrchardParams,
	PalmShadeParams, RiparianGeneralParams, RiparianMixParams, RiverineGreenParams,
	RollingOaksParams, ShamanhomeParams, SpottyBushesParams, StorytellersParams,
	StrangeOasisParams, TallGrassParams, TemperateLowerMassivesParams, TemperateMassivesParams,
	TradeWindsParams, TropicalThicketParams, TropicalTuftsParams, TropicalUndergrowthParams,
	UnendingJungleParams, VineyardParams, WanderingAcaciaParams, WildGrassParams,
	DEFAULT_GROVE_EXTENT_XZ,
};
use chico_vegetation_components::{
	spawn_lod_scene_host, spawn_vegetation_components, vegetation_bounds, VegetationComponents,
};
use lod::gen::LodScene;

/// Grove-tile radius from center (`[-radius, radius]` on each axis).
pub const VAST_GROVE_RADIUS: i32 = 10;

/// Plains tile span on XZ (matches default grove preview extent).
pub const VAST_GROVE_EXTENT_XZ: f32 = DEFAULT_GROVE_EXTENT_XZ;

/// Backward-compatible alias for orchard plains.
pub const VAST_ORCHARD_RADIUS: i32 = VAST_GROVE_RADIUS;

/// Kebab-case grove names accepted by `/show vast --grove-name`.
pub const VAST_GROVE_NAMES: &[&str] = &[
	"alpine",
	"arid-conifer-sapling",
	"braid-grass",
	"bush-scrub",
	"christmas-taiga",
	"common-tufts",
	"conifer-massives",
	"conifer-sapling",
	"date-grove",
	"dryland",
	"forlorn-savanna",
	"goettingen-follow",
	"high-bush",
	"jerrys-chaparral",
	"jungle-lower-massives",
	"jungle-massives",
	"leeward",
	"levantine-scrub",
	"low-bush",
	"monster-grass",
	"orchard",
	"palm-shade",
	"riparian-general",
	"riparian-mix",
	"riverine-green",
	"rolling-oaks",
	"shamanhome",
	"spotty-bushes",
	"storytellers",
	"strange-oasis",
	"tall-grass",
	"temperate-lower-massives",
	"temperate-massives",
	"trade-winds",
	"tropical-thicket",
	"tropical-tufts",
	"tropical-undergrowth",
	"unending-jungle",
	"vineyard",
	"wandering-acacia",
	"wild-grass",
];

/// Clap parser for `--grove-name`. Accepts aliases `vast-orchards` and `monster-grass-plains`.
pub fn parse_vast_grove_name(name: &str) -> Result<String, String> {
	let key = match name.trim().to_ascii_lowercase().as_str() {
		"vast-orchards" => "orchard".to_string(),
		"monster-grass-plains" => "monster-grass".to_string(),
		other => other.to_string(),
	};
	if VAST_GROVE_NAMES.contains(&key.as_str()) {
		Ok(key)
	} else {
		Err(format!("unknown grove {name:?}; expected one of: {}", VAST_GROVE_NAMES.join(", ")))
	}
}

fn tile_extents() -> impl Iterator<Item = GroveExtent> {
	(-VAST_GROVE_RADIUS..=VAST_GROVE_RADIUS).flat_map(|ix| {
		(-VAST_GROVE_RADIUS..=VAST_GROVE_RADIUS).map(move |iz| {
			let min =
				Vec3::new(ix as f32 * VAST_GROVE_EXTENT_XZ, 0.0, iz as f32 * VAST_GROVE_EXTENT_XZ);
			GroveExtent::new(min, min + Vec3::new(VAST_GROVE_EXTENT_XZ, 1.0, VAST_GROVE_EXTENT_XZ))
		})
	})
}

fn spawn_tiled_lod<T, F>(commands: &mut Commands, transform: Transform, mut build: F) -> Vec<Entity>
where
	T: LodScene + VegetationComponents + Component + Clone + Send + Sync + 'static,
	F: FnMut(GroveExtent) -> T,
{
	let axis = 2 * VAST_GROVE_RADIUS + 1;
	let mut entities = Vec::with_capacity((axis * axis) as usize);
	for extent in tile_extents() {
		let grove = build(extent);
		let bounds = grove
			.structural_lod()
			.map(|p| p.footprint_aabb())
			.unwrap_or_else(|| vegetation_bounds(&grove));
		entities.extend(spawn_lod_scene_host(commands, &grove, transform, bounds));
	}
	entities
}

fn spawn_tiled_components<T, F>(
	commands: &mut Commands,
	transform: Transform,
	mut build: F,
) -> Vec<Entity>
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
	F: FnMut(GroveExtent) -> T,
{
	let axis = 2 * VAST_GROVE_RADIUS + 1;
	let mut entities = Vec::with_capacity((axis * axis) as usize);
	for extent in tile_extents() {
		let grove = build(extent);
		let bounds = vegetation_bounds(&grove);
		entities.extend(spawn_vegetation_components(commands, &grove, transform, bounds));
	}
	entities
}

/// Spawn a centered `(2 × [`VAST_GROVE_RADIUS`] + 1)²` tile of default groves named `grove_name`.
pub fn spawn_vast_grove(
	commands: &mut Commands,
	transform: Transform,
	grove_name: &str,
) -> Result<Vec<Entity>, String> {
	let name = parse_vast_grove_name(grove_name)?;
	Ok(match name.as_str() {
		"alpine" => {
			spawn_tiled_lod(commands, transform, |e| AlpineParams::default().with_extent(e).build())
		}
		"arid-conifer-sapling" => spawn_tiled_lod(commands, transform, |e| {
			AridConiferSaplingParams::default().with_extent(e).build()
		}),
		"braid-grass" => spawn_tiled_components(commands, transform, |e| {
			BraidGrassParams::default().with_extent(e).build()
		}),
		"bush-scrub" => spawn_tiled_lod(commands, transform, |e| {
			BushScrubParams::default().with_extent(e).build()
		}),
		"christmas-taiga" => spawn_tiled_lod(commands, transform, |e| {
			ChristmasTaigaParams::default().with_extent(e).build()
		}),
		"common-tufts" => spawn_tiled_components(commands, transform, |e| {
			CommonTuftsParams::default().with_extent(e).build()
		}),
		"conifer-massives" => spawn_tiled_lod(commands, transform, |e| {
			ConiferMassivesParams::default().with_extent(e).build()
		}),
		"conifer-sapling" => spawn_tiled_lod(commands, transform, |e| {
			ConiferSaplingParams::default().with_extent(e).build()
		}),
		"date-grove" => spawn_tiled_lod(commands, transform, |e| {
			DateGroveParams::default().with_extent(e).build()
		}),
		"dryland" => spawn_tiled_lod(commands, transform, |e| {
			DrylandParams::default().with_extent(e).build()
		}),
		"forlorn-savanna" => spawn_tiled_lod(commands, transform, |e| {
			ForlornSavannaParams::default().with_extent(e).build()
		}),
		"goettingen-follow" => spawn_tiled_lod(commands, transform, |e| {
			GoettingenFollowParams::default().with_extent(e).build()
		}),
		"high-bush" => spawn_tiled_lod(commands, transform, |e| {
			HighBushParams::default().with_extent(e).build()
		}),
		"jerrys-chaparral" => spawn_tiled_lod(commands, transform, |e| {
			JerrysChaparralParams::default().with_extent(e).build()
		}),
		"jungle-lower-massives" => spawn_tiled_lod(commands, transform, |e| {
			JungleLowerMassivesParams::default().with_extent(e).build()
		}),
		"jungle-massives" => spawn_tiled_lod(commands, transform, |e| {
			JungleMassivesParams::default().with_extent(e).build()
		}),
		"leeward" => spawn_tiled_lod(commands, transform, |e| {
			LeewardParams::default().with_extent(e).build()
		}),
		"levantine-scrub" => spawn_tiled_lod(commands, transform, |e| {
			LevantineScrubParams::default().with_extent(e).build()
		}),
		"low-bush" => spawn_tiled_lod(commands, transform, |e| {
			LowBushParams::default().with_extent(e).build()
		}),
		"monster-grass" => spawn_tiled_components(commands, transform, |e| {
			MonsterGrassParams::default().with_extent(e).build()
		}),
		"orchard" => spawn_tiled_lod(commands, transform, |e| {
			OrchardParams::default().with_extent(e).build()
		}),
		"palm-shade" => spawn_tiled_lod(commands, transform, |e| {
			PalmShadeParams::default().with_extent(e).build()
		}),
		"riparian-general" => spawn_tiled_lod(commands, transform, |e| {
			RiparianGeneralParams::default().with_extent(e).build()
		}),
		"riparian-mix" => spawn_tiled_lod(commands, transform, |e| {
			RiparianMixParams::default().with_extent(e).build()
		}),
		"riverine-green" => spawn_tiled_lod(commands, transform, |e| {
			RiverineGreenParams::default().with_extent(e).build()
		}),
		"rolling-oaks" => spawn_tiled_lod(commands, transform, |e| {
			RollingOaksParams::default().with_extent(e).build()
		}),
		"shamanhome" => spawn_tiled_lod(commands, transform, |e| {
			ShamanhomeParams::default().with_extent(e).build()
		}),
		"spotty-bushes" => spawn_tiled_lod(commands, transform, |e| {
			SpottyBushesParams::default().with_extent(e).build()
		}),
		"storytellers" => spawn_tiled_lod(commands, transform, |e| {
			StorytellersParams::default().with_extent(e).build()
		}),
		"strange-oasis" => spawn_tiled_lod(commands, transform, |e| {
			StrangeOasisParams::default().with_extent(e).build()
		}),
		"tall-grass" => spawn_tiled_components(commands, transform, |e| {
			TallGrassParams::default().with_extent(e).build()
		}),
		"temperate-lower-massives" => spawn_tiled_lod(commands, transform, |e| {
			TemperateLowerMassivesParams::default().with_extent(e).build()
		}),
		"temperate-massives" => spawn_tiled_lod(commands, transform, |e| {
			TemperateMassivesParams::default().with_extent(e).build()
		}),
		"trade-winds" => spawn_tiled_lod(commands, transform, |e| {
			TradeWindsParams::default().with_extent(e).build()
		}),
		"tropical-thicket" => spawn_tiled_lod(commands, transform, |e| {
			TropicalThicketParams::default().with_extent(e).build()
		}),
		"tropical-tufts" => spawn_tiled_components(commands, transform, |e| {
			TropicalTuftsParams::default().with_extent(e).build()
		}),
		"tropical-undergrowth" => spawn_tiled_lod(commands, transform, |e| {
			TropicalUndergrowthParams::default().with_extent(e).build()
		}),
		"unending-jungle" => spawn_tiled_lod(commands, transform, |e| {
			UnendingJungleParams::default().with_extent(e).build()
		}),
		"vineyard" => spawn_tiled_lod(commands, transform, |e| {
			VineyardParams::default().with_extent(e).build()
		}),
		"wandering-acacia" => spawn_tiled_lod(commands, transform, |e| {
			WanderingAcaciaParams::default().with_extent(e).build()
		}),
		"wild-grass" => spawn_tiled_components(commands, transform, |e| {
			WildGrassParams::default().with_extent(e).build()
		}),
		other => return Err(format!("unhandled grove {other:?}")),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vast_is_radius_ten_centered() {
		assert_eq!(VAST_GROVE_RADIUS, 10);
		assert_eq!((-VAST_GROVE_RADIUS..=VAST_GROVE_RADIUS).count(), 21);
		assert!((VAST_GROVE_EXTENT_XZ - DEFAULT_GROVE_EXTENT_XZ).abs() < 1e-5);
	}

	#[test]
	fn parse_vast_grove_name_accepts_aliases() {
		assert_eq!(parse_vast_grove_name("orchard").unwrap(), "orchard");
		assert_eq!(parse_vast_grove_name("vast-orchards").unwrap(), "orchard");
		assert_eq!(parse_vast_grove_name("Goettingen-Follow").unwrap(), "goettingen-follow");
		assert_eq!(parse_vast_grove_name("monster-grass-plains").unwrap(), "monster-grass");
		assert!(parse_vast_grove_name("not-a-grove").is_err());
	}
}
