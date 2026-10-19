//! Typed bags for the `mythical_chain` [`MaterialRef`](material_ref::MaterialRef) contract.

use bevy::prelude::Color;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::flow::ChainFlowStyle;

/// Recipe claimed by [`crate::ChainMaterialLib`].
pub const RECIPE_MYTHICAL_CHAIN: &str = "mythical_chain";

/// Palette slot: intact metal.
pub const PALETTE_METAL: usize = 0;
/// Palette slot: energy glow.
pub const PALETTE_GLOW: usize = 1;
/// Palette slot: damage / ash.
pub const PALETTE_DAMAGE: usize = 2;
/// Palette slot: ember / near-break heat.
pub const PALETTE_EMBER: usize = 3;

/// Scalar 0: remaining durability (`1` intact, `0` destroyed).
pub const SCALAR_INTACT: usize = 0;
/// Scalar 1: glow gain.
pub const SCALAR_GLOW_INTENSITY: usize = 1;
/// Scalar 2: break / fade / release progress.
pub const SCALAR_FADE: usize = 2;
/// Scalar 3: damage amount. `0` means derive `1 - intact`.
pub const SCALAR_DAMAGE: usize = 3;
/// Scalar 4: link-count hint for the UV.y fallback.
pub const SCALAR_LINK_COUNT: usize = 4;
/// Scalar 5: decorative energy-conduit frequency (not structural damage).
pub const SCALAR_CONDUIT_FREQUENCY: usize = 5;
/// Scalar 6: blend toward vertex-color G. Prefer [`crate::OwnedChainMaterial`]
/// + the per-link uniform array; this scalar is the legacy mesh-color path.
pub const SCALAR_LOCALIZED_BLEND: usize = 6;
/// Scalar 7: reserved. Older recipes stored a pulse rate here; it is ignored.
pub const SCALAR_RESERVED: usize = 7;
/// Scalar 8: signed flow speed (metres / second).
pub const SCALAR_FLOW_SPEED: usize = 8;
/// Scalar 9: flow wavelength (metres, `> 0`).
pub const SCALAR_FLOW_WAVELENGTH: usize = 9;
/// Scalar 10: flow baseline energy.
pub const SCALAR_FLOW_BASELINE: usize = 10;
/// Scalar 11: flow peak energy.
pub const SCALAR_FLOW_PEAK: usize = 11;
/// Scalar 12: halo shell width (metres).
pub const SCALAR_FLOW_HALO_WIDTH: usize = 12;
/// Scalar 13: halo gain.
pub const SCALAR_FLOW_HALO_GAIN: usize = 13;

/// Packed float count written by [`MythicalChainScalars::to_array`].
pub const MYTHICAL_CHAIN_SCALAR_COUNT: usize = 14;

/// Default metal: blue-gray body, not scorched iron.
pub const DEFAULT_METAL: Color = Color::srgb(0.34, 0.36, 0.40);
/// Default energy: saturated turquoise.
pub const DEFAULT_GLOW: Color = Color::srgb(0.04, 0.78, 0.68);
/// Default ash / broken-edge soil. Cool, not rust-orange.
pub const DEFAULT_DAMAGE: Color = Color::srgb(0.26, 0.22, 0.20);
/// Optional ember accent. Used sparingly on broken lips.
pub const DEFAULT_EMBER: Color = Color::srgb(0.70, 0.30, 0.10);

/// Authored glow gain. Kept modest so emission is a highlight, not a cage.
pub const DEFAULT_GLOW_INTENSITY: f32 = 2.2;
/// Authored decorative conduit frequency.
pub const DEFAULT_CONDUIT_FREQUENCY: f32 = 2.2;
/// Authored link-count hint (matches the placeholder mesh).
pub const DEFAULT_LINK_COUNT: f32 = 14.0;
/// Authored noise seed.
pub const DEFAULT_SEED: i32 = 11;
/// Authored noise frequency (low, for painted islands).
pub const DEFAULT_NOISE_FREQUENCY: f32 = 0.55;

/// Preview / encounter states. Same mesh, different scalars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MythicalChainState {
	/// Full metal, quiet conduits, no missing chunks.
	Intact,
	/// Mid structural loss: obvious bites, energy still travels.
	Damaged,
	/// Late structural loss: substantial gaps, reserved plates remain.
	NearBreaking,
	/// Gameplay-owned release. Fade removes body and halo.
	Released,
}

impl MythicalChainState {
	pub const fn intact(self) -> f32 {
		match self {
			Self::Intact => 1.0,
			Self::Damaged => 0.50,
			Self::NearBreaking => 0.22,
			Self::Released => 0.0,
		}
	}

	pub const fn fade(self) -> f32 {
		match self {
			Self::Intact | Self::Damaged | Self::NearBreaking => 0.0,
			Self::Released => 1.0,
		}
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Intact => "intact",
			Self::Damaged => "damaged",
			Self::NearBreaking => "near-breaking",
			Self::Released => "released",
		}
	}

	pub fn from_label(label: &str) -> Option<Self> {
		match label {
			"intact" => Some(Self::Intact),
			"damaged" => Some(Self::Damaged),
			"near-breaking" | "near_breaking" | "breaking" => Some(Self::NearBreaking),
			"released" | "broken" | "fade" => Some(Self::Released),
			_ => None,
		}
	}
}

/// Packed scalar bag. Index meaning is documented in the crate README.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MythicalChainScalars {
	pub intact: f32,
	pub glow_intensity: f32,
	pub fade: f32,
	pub damage: f32,
	pub link_count: f32,
	pub conduit_frequency: f32,
	pub localized_blend: f32,
	pub flow: ChainFlowStyle,
}

impl Default for MythicalChainScalars {
	fn default() -> Self {
		Self::for_state(MythicalChainState::Intact)
	}
}

impl MythicalChainScalars {
	pub fn for_state(state: MythicalChainState) -> Self {
		Self {
			intact: state.intact(),
			glow_intensity: DEFAULT_GLOW_INTENSITY,
			fade: state.fade(),
			damage: 0.0,
			link_count: DEFAULT_LINK_COUNT,
			conduit_frequency: DEFAULT_CONDUIT_FREQUENCY,
			localized_blend: 0.0,
			flow: ChainFlowStyle::default(),
		}
	}

	pub fn with_intact(mut self, intact: f32) -> Self {
		self.intact = intact;
		self
	}

	pub fn with_glow_intensity(mut self, glow_intensity: f32) -> Self {
		self.glow_intensity = glow_intensity;
		self
	}

	pub fn with_fade(mut self, fade: f32) -> Self {
		self.fade = fade;
		self
	}

	pub fn with_damage(mut self, damage: f32) -> Self {
		self.damage = damage;
		self
	}

	pub fn with_link_count(mut self, link_count: f32) -> Self {
		self.link_count = link_count;
		self
	}

	pub fn with_localized_blend(mut self, localized_blend: f32) -> Self {
		self.localized_blend = localized_blend;
		self
	}

	pub fn with_flow(mut self, flow: ChainFlowStyle) -> Self {
		self.flow = flow;
		self
	}

	pub fn clamped(self) -> Self {
		Self {
			intact: self.intact.clamp(0.0, 1.0),
			glow_intensity: self.glow_intensity.clamp(0.0, 8.0),
			fade: self.fade.clamp(0.0, 1.0),
			damage: self.damage.clamp(0.0, 1.0),
			link_count: self.link_count.clamp(1.0, 256.0),
			conduit_frequency: self.conduit_frequency.clamp(0.5, 24.0),
			localized_blend: self.localized_blend.clamp(0.0, 1.0),
			flow: self.flow.clamped(),
		}
	}

	/// Explicit `damage` if authored, otherwise `1 - intact`.
	pub fn effective_damage(self) -> f32 {
		let clamped = self.clamped();
		if clamped.damage > 1e-5 {
			clamped.damage
		} else {
			1.0 - clamped.intact
		}
	}

	pub fn to_array(self) -> [f32; MYTHICAL_CHAIN_SCALAR_COUNT] {
		let clamped = self.clamped();
		[
			clamped.intact,
			clamped.glow_intensity,
			clamped.fade,
			clamped.damage,
			clamped.link_count,
			clamped.conduit_frequency,
			clamped.localized_blend,
			0.0,
			clamped.flow.speed_mps,
			clamped.flow.wavelength_m,
			clamped.flow.baseline,
			clamped.flow.peak,
			clamped.flow.halo_width_m,
			clamped.flow.halo_gain,
		]
	}

	/// Read every authored slot that is present. Slices shorter than
	/// [`MYTHICAL_CHAIN_SCALAR_COUNT`] keep [`ChainFlowStyle`] defaults so an
	/// old 8-float recipe does not treat reserved slot 7 as a flow speed.
	pub fn from_slice(values: &[f32]) -> Self {
		let flow = if values.len() > SCALAR_FLOW_SPEED {
			ChainFlowStyle {
				speed_mps: values
					.get(SCALAR_FLOW_SPEED)
					.copied()
					.unwrap_or(ChainFlowStyle::DEFAULT_SPEED_MPS),
				wavelength_m: values
					.get(SCALAR_FLOW_WAVELENGTH)
					.copied()
					.unwrap_or(ChainFlowStyle::DEFAULT_WAVELENGTH_M),
				baseline: values
					.get(SCALAR_FLOW_BASELINE)
					.copied()
					.unwrap_or(ChainFlowStyle::DEFAULT_BASELINE),
				peak: values.get(SCALAR_FLOW_PEAK).copied().unwrap_or(ChainFlowStyle::DEFAULT_PEAK),
				halo_width_m: values
					.get(SCALAR_FLOW_HALO_WIDTH)
					.copied()
					.unwrap_or(ChainFlowStyle::DEFAULT_HALO_WIDTH_M),
				halo_gain: values
					.get(SCALAR_FLOW_HALO_GAIN)
					.copied()
					.unwrap_or(ChainFlowStyle::DEFAULT_HALO_GAIN),
			}
		} else {
			ChainFlowStyle::default()
		};
		Self {
			intact: values.get(SCALAR_INTACT).copied().unwrap_or(1.0),
			glow_intensity: values
				.get(SCALAR_GLOW_INTENSITY)
				.copied()
				.unwrap_or(DEFAULT_GLOW_INTENSITY),
			fade: values.get(SCALAR_FADE).copied().unwrap_or(0.0),
			damage: values.get(SCALAR_DAMAGE).copied().unwrap_or(0.0),
			link_count: values.get(SCALAR_LINK_COUNT).copied().unwrap_or(DEFAULT_LINK_COUNT),
			conduit_frequency: values
				.get(SCALAR_CONDUIT_FREQUENCY)
				.copied()
				.unwrap_or(DEFAULT_CONDUIT_FREQUENCY),
			localized_blend: values.get(SCALAR_LOCALIZED_BLEND).copied().unwrap_or(0.0),
			flow,
		}
		.clamped()
	}
}

/// Authored palette for the mythical look.
pub fn mythical_chain_palette() -> [Color; 4] {
	[DEFAULT_METAL, DEFAULT_GLOW, DEFAULT_DAMAGE, DEFAULT_EMBER]
}

/// Authored noise. `seed` is the per-chain identity A06 should keep stable.
pub fn mythical_chain_noise(seed: i32) -> NoiseParams {
	NoiseParams::from_scalar(seed as f32, DEFAULT_NOISE_FREQUENCY, 1.0, 2)
}

/// `mythical_chain` recipe at an authored state.
pub fn mythical_chain_ref_for(state: MythicalChainState) -> MaterialRef {
	mythical_chain_ref_with(MythicalChainScalars::for_state(state), DEFAULT_SEED)
}

/// Intact authored recipe.
pub fn mythical_chain_ref() -> MaterialRef {
	mythical_chain_ref_for(MythicalChainState::Intact)
}

/// Build a recipe from a typed scalar bag and seed.
pub fn mythical_chain_ref_with(scalars: MythicalChainScalars, seed: i32) -> MaterialRef {
	MaterialRef::named(RECIPE_MYTHICAL_CHAIN)
		.with_palette(mythical_chain_palette())
		.with_noise(mythical_chain_noise(seed))
		.with_scalars(scalars.to_array())
}

/// True when `name` is the mythical-chain recipe.
pub fn is_mythical_chain_recipe(name: &str) -> bool {
	name == RECIPE_MYTHICAL_CHAIN
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn to_array_round_trips_flow_fields() -> anyhow::Result<()> {
		let scalars = MythicalChainScalars::default().with_flow(ChainFlowStyle {
			speed_mps: -1.1,
			wavelength_m: 2.4,
			baseline: 0.2,
			peak: 0.9,
			halo_width_m: 0.013,
			halo_gain: 0.4,
		});
		let packed = scalars.to_array();
		assert_eq!(packed.len(), MYTHICAL_CHAIN_SCALAR_COUNT);
		assert_eq!(packed[SCALAR_RESERVED], 0.0);
		let back = MythicalChainScalars::from_slice(&packed);
		assert!((back.flow.speed_mps - (-1.1)).abs() < 1e-5);
		assert!((back.flow.wavelength_m - 2.4).abs() < 1e-5);
		assert!((back.flow.baseline - 0.2).abs() < 1e-5);
		assert!((back.flow.peak - 0.9).abs() < 1e-5);
		assert!((back.flow.halo_width_m - 0.013).abs() < 1e-5);
		assert!((back.flow.halo_gain - 0.4).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn short_legacy_slice_keeps_flow_defaults() -> anyhow::Result<()> {
		let legacy = [1.0, 2.2, 0.0, 0.0, 14.0, 2.2, 0.0, 1.4];
		let scalars = MythicalChainScalars::from_slice(&legacy);
		assert!((scalars.flow.speed_mps - ChainFlowStyle::DEFAULT_SPEED_MPS).abs() < 1e-5);
		assert!((scalars.flow.wavelength_m - ChainFlowStyle::DEFAULT_WAVELENGTH_M).abs() < 1e-5);
		Ok(())
	}
}
