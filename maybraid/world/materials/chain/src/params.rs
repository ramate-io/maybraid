//! Typed bags for the `mythical_chain` [`MaterialRef`](material_ref::MaterialRef) contract.

use bevy::prelude::Color;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

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
/// Scalar 5: UV-space crack frequency.
pub const SCALAR_CRACK_FREQUENCY: usize = 5;
/// Scalar 6: blend toward vertex-color G. Prefer [`crate::OwnedChainMaterial`]
/// + the per-link uniform array; this scalar is the legacy mesh-color path.
pub const SCALAR_LOCALIZED_BLEND: usize = 6;
/// Scalar 7: glow / ember pulse rate.
pub const SCALAR_PULSE_RATE: usize = 7;

/// Default metal.
pub const DEFAULT_METAL: Color = Color::srgb(0.22, 0.16, 0.12);
/// Default energy glow.
pub const DEFAULT_GLOW: Color = Color::srgb(0.62, 0.90, 1.0);
/// Default rust / ash.
pub const DEFAULT_DAMAGE: Color = Color::srgb(0.20, 0.08, 0.05);
/// Default ember.
pub const DEFAULT_EMBER: Color = Color::srgb(1.0, 0.42, 0.10);

/// Authored glow gain.
pub const DEFAULT_GLOW_INTENSITY: f32 = 5.2;
/// Authored crack frequency.
pub const DEFAULT_CRACK_FREQUENCY: f32 = 6.0;
/// Authored pulse rate.
pub const DEFAULT_PULSE_RATE: f32 = 1.4;
/// Authored link-count hint (matches the placeholder mesh).
pub const DEFAULT_LINK_COUNT: f32 = 14.0;
/// Authored noise seed.
pub const DEFAULT_SEED: i32 = 11;
/// Authored noise frequency.
pub const DEFAULT_NOISE_FREQUENCY: f32 = 2.4;

/// Preview / encounter states. Same mesh, different scalars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MythicalChainState {
	/// Full glow, clean metal.
	Intact,
	/// Cracks and dimmer veins. Global durability still above a break.
	Damaged,
	/// Ember, flicker, weak-link holes.
	NearBreaking,
	/// Dissolve / leftover sparks after release.
	Released,
}

impl MythicalChainState {
	pub const fn intact(self) -> f32 {
		match self {
			Self::Intact => 1.0,
			Self::Damaged => 0.55,
			Self::NearBreaking => 0.18,
			Self::Released => 0.0,
		}
	}

	pub const fn fade(self) -> f32 {
		match self {
			Self::Intact | Self::Damaged => 0.0,
			Self::NearBreaking => 0.15,
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
	pub crack_frequency: f32,
	pub localized_blend: f32,
	pub pulse_rate: f32,
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
			crack_frequency: DEFAULT_CRACK_FREQUENCY,
			localized_blend: 0.0,
			pulse_rate: DEFAULT_PULSE_RATE,
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

	pub fn clamped(self) -> Self {
		Self {
			intact: self.intact.clamp(0.0, 1.0),
			glow_intensity: self.glow_intensity.clamp(0.0, 8.0),
			fade: self.fade.clamp(0.0, 1.0),
			damage: self.damage.clamp(0.0, 1.0),
			link_count: self.link_count.clamp(1.0, 256.0),
			crack_frequency: self.crack_frequency.clamp(0.5, 24.0),
			localized_blend: self.localized_blend.clamp(0.0, 1.0),
			pulse_rate: self.pulse_rate.clamp(0.0, 8.0),
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

	pub fn to_array(self) -> [f32; 8] {
		let clamped = self.clamped();
		[
			clamped.intact,
			clamped.glow_intensity,
			clamped.fade,
			clamped.damage,
			clamped.link_count,
			clamped.crack_frequency,
			clamped.localized_blend,
			clamped.pulse_rate,
		]
	}

	pub fn from_slice(values: &[f32]) -> Self {
		Self {
			intact: values.get(SCALAR_INTACT).copied().unwrap_or(1.0),
			glow_intensity: values
				.get(SCALAR_GLOW_INTENSITY)
				.copied()
				.unwrap_or(DEFAULT_GLOW_INTENSITY),
			fade: values.get(SCALAR_FADE).copied().unwrap_or(0.0),
			damage: values.get(SCALAR_DAMAGE).copied().unwrap_or(0.0),
			link_count: values.get(SCALAR_LINK_COUNT).copied().unwrap_or(DEFAULT_LINK_COUNT),
			crack_frequency: values
				.get(SCALAR_CRACK_FREQUENCY)
				.copied()
				.unwrap_or(DEFAULT_CRACK_FREQUENCY),
			localized_blend: values.get(SCALAR_LOCALIZED_BLEND).copied().unwrap_or(0.0),
			pulse_rate: values.get(SCALAR_PULSE_RATE).copied().unwrap_or(DEFAULT_PULSE_RATE),
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
