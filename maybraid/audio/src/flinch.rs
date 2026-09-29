//! Character flinch vocals. A [`FlinchProfile`] picks a [`GruntStyle`]; damage
//! play selects a numbered variant without repeating the last one.

use bevy::prelude::*;

use crate::asset::AudioClip;
use crate::backend::Audio;
use crate::mixer::{AudioBus, Mixer};
use crate::spatial::SpatialOneShot;

/// Inverse of the oddio zero-attenuation radius (meters).
pub const GRUNT_SPATIAL_SCALE: f32 = 0.1;
pub const GRUNT_SPATIAL_RADIUS: f32 = 1.0 / GRUNT_SPATIAL_SCALE;
pub const GRUNT_VOLUME: f32 = 1.0;

/// Authored grunt family. Variants are `{slug}_{index:03}.wav`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GruntStyle {
	#[default]
	Man,
}

impl GruntStyle {
	pub const VALUES: [Self; 1] = [Self::Man];

	pub const fn slug(self) -> &'static str {
		match self {
			Self::Man => "man",
		}
	}

	pub const fn variant_count(self) -> usize {
		match self {
			Self::Man => 3,
		}
	}

	/// Asset path for a 0-based variant.
	pub fn path(self, variant: usize) -> String {
		let index = variant % self.variant_count() + 1;
		format!("sound-effects/character/damage/{}_{index:03}.wav", self.slug())
	}
}

/// Per-character flinch recipe. More fields will land here; grunt style is first.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlinchProfile {
	pub grunt: GruntStyle,
}

impl FlinchProfile {
	pub const fn new(grunt: GruntStyle) -> Self {
		Self { grunt }
	}
}

/// Noisy pick state so the same variant is not replayed back-to-back.
#[derive(Component, Clone, Copy, Debug)]
pub struct FlinchState {
	pub noise: u64,
	pub last_grunt: Option<u8>,
}

impl FlinchState {
	pub fn seeded(seed: u64) -> Self {
		Self { noise: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed }, last_grunt: None }
	}

	/// Advance the xorshift and pick among `count`, skipping [`Self::last_grunt`].
	pub fn pick_grunt(&mut self, count: usize) -> usize {
		let last = self.last_grunt.map(|index| index as usize);
		let variant = pick_variant(count, last, &mut self.noise);
		self.last_grunt = Some(variant as u8);
		variant
	}
}

/// Loaded grunt clips, keyed by style.
#[derive(Resource)]
pub struct FlinchSounds {
	man: [Handle<AudioClip>; 3],
}

impl FlinchSounds {
	pub fn load(assets: &AssetServer) -> Self {
		Self {
			man: [
				assets.load(GruntStyle::Man.path(0)),
				assets.load(GruntStyle::Man.path(1)),
				assets.load(GruntStyle::Man.path(2)),
			],
		}
	}

	pub fn grunt(&self, style: GruntStyle, variant: usize) -> &Handle<AudioClip> {
		let count = style.variant_count();
		let index = if count == 0 { 0 } else { variant % count };
		match style {
			GruntStyle::Man => &self.man[index],
		}
	}

	pub fn play(
		&self,
		commands: &mut Commands,
		profile: FlinchProfile,
		state: &mut FlinchState,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		let variant = state.pick_grunt(profile.grunt.variant_count());
		audio.play_or_queue(
			commands,
			self.grunt(profile.grunt, variant),
			clips,
			SpatialOneShot::at(point)
				.radius(GRUNT_SPATIAL_RADIUS)
				.gain(GRUNT_VOLUME)
				.bus(AudioBus::Voices),
			mixer,
			listener,
			"flinch-grunt",
		);
	}
}

impl FromWorld for FlinchSounds {
	fn from_world(world: &mut World) -> Self {
		let assets = world.resource::<AssetServer>();
		Self::load(assets)
	}
}

pub(crate) fn setup_flinch_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(FlinchSounds::load(&assets));
}

/// Xorshift pick. When `last` is in range, that slot is excluded.
pub fn pick_variant(count: usize, last: Option<usize>, noise: &mut u64) -> usize {
	if count <= 1 {
		return 0;
	}
	let mixed = next_u64(noise);
	match last.filter(|last| *last < count) {
		Some(last) => {
			let skip = (mixed as usize) % (count - 1);
			if skip < last { skip } else { skip + 1 }
		}
		None => (mixed as usize) % count,
	}
}

fn next_u64(noise: &mut u64) -> u64 {
	let mut x = if *noise == 0 { 0x9E37_79B9_7F4A_7C15 } else { *noise };
	x ^= x >> 12;
	x ^= x << 25;
	x ^= x >> 27;
	*noise = x;
	x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	#[test]
	fn man_paths_are_numbered_from_one() {
		assert_eq!(GruntStyle::Man.path(0), "sound-effects/character/damage/man_001.wav");
		assert_eq!(GruntStyle::Man.path(2), "sound-effects/character/damage/man_003.wav");
		assert_eq!(GruntStyle::Man.path(3), "sound-effects/character/damage/man_001.wav");
	}

	#[test]
	fn authored_man_grunts_are_mono_assets() {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
		for variant in 0..GruntStyle::Man.variant_count() {
			let file = root.join(GruntStyle::Man.path(variant));
			let bytes = match std::fs::read(&file) {
				Ok(bytes) => bytes,
				Err(_) => panic!("{}", file.display()),
			};
			assert!(
				crate::decode_wav_mono(&bytes).is_ok(),
				"{} must be authored mono",
				file.display()
			);
		}
	}

	#[test]
	fn noisy_pick_skips_the_last_variant() {
		let mut noise = 1_u64;
		for last in 0..3 {
			for _ in 0..32 {
				let next = pick_variant(3, Some(last), &mut noise);
				assert_ne!(next, last);
				assert!(next < 3);
			}
		}
	}

	#[test]
	fn first_pick_uses_the_full_set() {
		let mut seen = [false; 3];
		for seed in 1_u64..=48 {
			let mut noise = seed;
			seen[pick_variant(3, None, &mut noise)] = true;
		}
		assert!(seen.iter().all(|hit| *hit));
	}

	#[test]
	fn flinch_state_remembers_the_grunt() {
		let mut state = FlinchState::seeded(7);
		let first = state.pick_grunt(3);
		let second = state.pick_grunt(3);
		assert_ne!(first, second);
		assert_eq!(state.last_grunt, Some(second as u8));
	}
}
