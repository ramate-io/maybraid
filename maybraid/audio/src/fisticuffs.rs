//! Fisticuffs strike effort vocals. [`FisticuffsStrikeAudioState`] picks numbered
//! variants per combo step without repeating the last clip, with small pitch jitter.

use bevy::prelude::*;

use crate::asset::AudioClip;
use crate::backend::Audio;
use crate::flinch::{pick_variant, GRUNT_SPATIAL_RADIUS, GRUNT_VOLUME};
use crate::mixer::{AudioBus, Mixer};
use crate::spatial::SpatialOneShot;

/// Combo buckets rotate while the player chains jabs.
pub const COMBO_STEP_COUNT: usize = 3;
/// Variants per combo step.
pub const EFFORT_VARIANT_COUNT: usize = 3;

pub const EFFORT_PITCH_MIN: f32 = 0.97;
pub const EFFORT_PITCH_MAX: f32 = 1.03;

/// Authored effort family. Variants are `effort_man_{combo}_{index:03}.wav`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EffortStyle {
	#[default]
	Man,
}

impl EffortStyle {
	pub const fn slug(self) -> &'static str {
		match self {
			Self::Man => "man",
		}
	}

	pub fn path(self, combo_step: usize, variant: usize) -> String {
		let combo = combo_step % COMBO_STEP_COUNT + 1;
		let index = variant % EFFORT_VARIANT_COUNT + 1;
		format!(
			"sound-effects/character/fisticuffs/effort_{}_{combo:01}_{index:03}.wav",
			self.slug()
		)
	}
}

/// Noisy pick state for strike effort clips.
#[derive(Component, Clone, Copy, Debug)]
pub struct FisticuffsStrikeAudioState {
	pub noise: u64,
	pub last_variant: Option<u8>,
}

impl FisticuffsStrikeAudioState {
	pub fn seeded(seed: u64) -> Self {
		Self {
			noise: if seed == 0 { 0xD1B5_4A32_D192_ED03 } else { seed },
			last_variant: None,
		}
	}

	pub fn pick_variant(&mut self) -> usize {
		let last = self.last_variant.map(|index| index as usize);
		let variant = pick_variant(EFFORT_VARIANT_COUNT, last, &mut self.noise);
		self.last_variant = Some(variant as u8);
		variant
	}

	pub fn pitch_jitter(&mut self) -> f32 {
		let mixed = advance_noise(&mut self.noise);
		let t = (mixed % 10_000) as f32 / 10_000.0;
		EFFORT_PITCH_MIN + t * (EFFORT_PITCH_MAX - EFFORT_PITCH_MIN)
	}
}

fn advance_noise(noise: &mut u64) -> u64 {
	let mut x = if *noise == 0 { 0x9E37_79B9_7F4A_7C15 } else { *noise };
	x ^= x >> 12;
	x ^= x << 25;
	x ^= x >> 27;
	*noise = x;
	x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

/// Loaded strike-effort clips for each combo step × variant.
#[derive(Resource)]
pub struct FisticuffsStrikeSounds {
	man: [[Handle<AudioClip>; EFFORT_VARIANT_COUNT]; COMBO_STEP_COUNT],
}

impl FisticuffsStrikeSounds {
	/// Test / harness helper when every slot should share one loaded clip.
	pub fn with_uniform_clip(handle: Handle<AudioClip>) -> Self {
		Self { man: std::array::from_fn(|_| std::array::from_fn(|_| handle.clone())) }
	}

	pub fn load(assets: &AssetServer) -> Self {
		let style = EffortStyle::Man;
		let man = std::array::from_fn(|combo| {
			std::array::from_fn(|variant| assets.load(style.path(combo, variant)))
		});
		Self { man }
	}

	pub fn clip(&self, combo_step: usize, variant: usize) -> &Handle<AudioClip> {
		let combo = combo_step % COMBO_STEP_COUNT;
		let index = variant % EFFORT_VARIANT_COUNT;
		&self.man[combo][index]
	}

	pub fn play_strike(
		&self,
		commands: &mut Commands,
		combo_step: u8,
		state: &mut FisticuffsStrikeAudioState,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		let variant = state.pick_variant();
		let rate = state.pitch_jitter();
		audio.play_or_queue(
			commands,
			self.clip(combo_step as usize, variant),
			clips,
			SpatialOneShot::at(point)
				.radius(GRUNT_SPATIAL_RADIUS)
				.gain(GRUNT_VOLUME)
				.bus(AudioBus::Voices)
				.playback_rate(rate),
			mixer,
			listener,
			"fisticuffs-strike-grunt",
		);
	}
}

impl FromWorld for FisticuffsStrikeSounds {
	fn from_world(world: &mut World) -> Self {
		let assets = world.resource::<AssetServer>();
		Self::load(assets)
	}
}

pub(crate) fn setup_fisticuffs_strike_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(FisticuffsStrikeSounds::load(&assets));
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	#[test]
	fn effort_paths_are_numbered_per_combo() {
		assert_eq!(
			EffortStyle::Man.path(0, 0),
			"sound-effects/character/fisticuffs/effort_man_1_001.wav"
		);
		assert_eq!(
			EffortStyle::Man.path(2, 1),
			"sound-effects/character/fisticuffs/effort_man_3_002.wav"
		);
	}

	#[test]
	fn placeholder_efforts_are_mono_assets() {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
		for combo in 0..COMBO_STEP_COUNT {
			for variant in 0..EFFORT_VARIANT_COUNT {
				let file = root.join(EffortStyle::Man.path(combo, variant));
				let bytes = std::fs::read(&file).unwrap_or_else(|_| panic!("{}", file.display()));
				assert!(
					crate::decode_wav_mono(&bytes).is_ok(),
					"{} must be authored mono",
					file.display()
				);
			}
		}
	}

	#[test]
	fn variant_pick_is_deterministic_for_a_seed() {
		let mut a = FisticuffsStrikeAudioState::seeded(42);
		let mut b = FisticuffsStrikeAudioState::seeded(42);
		for _ in 0..8 {
			assert_eq!(a.pick_variant(), b.pick_variant());
			assert!((a.pitch_jitter() - b.pitch_jitter()).abs() < 1e-6);
		}
	}

	#[test]
	fn variant_pick_skips_back_to_back_repeats() {
		let mut state = FisticuffsStrikeAudioState::seeded(9);
		let first = state.pick_variant();
		let second = state.pick_variant();
		assert_ne!(first, second);
	}
}
