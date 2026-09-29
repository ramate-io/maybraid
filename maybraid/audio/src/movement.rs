//! Character movement foley. Numbered variants live under
//! `sound-effects/character/movement/`.

use bevy::prelude::*;

use crate::asset::AudioClip;
use crate::backend::Audio;
use crate::flinch::pick_variant;
use crate::mixer::{AudioBus, Mixer};
use crate::spatial::SpatialOneShot;

/// Inverse of the oddio zero-attenuation radius (meters).
pub const FOOTSTEP_SPATIAL_SCALE: f32 = 0.4;
pub const FOOTSTEP_SPATIAL_RADIUS: f32 = 1.0 / FOOTSTEP_SPATIAL_SCALE;
pub const FOOTSTEP_VOLUME: f32 = 1.6;
pub const CHANGE_ITEM_SPATIAL_SCALE: f32 = 0.45;
pub const CHANGE_ITEM_SPATIAL_RADIUS: f32 = 1.0 / CHANGE_ITEM_SPATIAL_SCALE;
pub const CHANGE_ITEM_VOLUME: f32 = 1.4;

/// Authored movement clip family. Variants are `{slug}_{index:03}.wav`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementClip {
	Footstep,
	ChangeItem,
}

impl MovementClip {
	pub const VALUES: [Self; 2] = [Self::Footstep, Self::ChangeItem];

	pub const fn slug(self) -> &'static str {
		match self {
			Self::Footstep => "footstep",
			Self::ChangeItem => "change_item",
		}
	}

	pub const fn variant_count(self) -> usize {
		match self {
			Self::Footstep | Self::ChangeItem => 1,
		}
	}

	/// Asset path for a 0-based variant.
	pub fn path(self, variant: usize) -> String {
		let index = variant % self.variant_count() + 1;
		format!("sound-effects/character/movement/{}_{index:03}.wav", self.slug())
	}
}

/// Noisy pick state so a later variant set does not replay the last step.
#[derive(Component, Clone, Copy, Debug)]
pub struct MovementState {
	pub phase: f32,
	pub noise: u64,
	pub last_foot: Option<u8>,
}

impl MovementState {
	pub fn seeded(seed: u64) -> Self {
		Self {
			phase: 0.0,
			noise: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
			last_foot: None,
		}
	}

	pub fn pick_foot(&mut self, count: usize) -> usize {
		let last = self.last_foot.map(|index| index as usize);
		let variant = pick_variant(count, last, &mut self.noise);
		self.last_foot = Some(variant as u8);
		variant
	}

	/// Advance gait by `rate` steps/sec. Returns how many footfalls landed this tick.
	pub fn take_steps(&mut self, rate: f32, dt: f32) -> u32 {
		if rate <= 0.0 || dt <= 0.0 {
			self.phase = 0.0;
			return 0;
		}
		self.phase += rate * dt;
		let mut steps = 0;
		while self.phase >= 1.0 {
			self.phase -= 1.0;
			steps += 1;
			if steps > 4 {
				self.phase = self.phase.fract();
				break;
			}
		}
		steps
	}
}

/// Loaded movement clips.
#[derive(Resource)]
pub struct MovementSounds {
	footstep: [Handle<AudioClip>; 1],
	change_item: [Handle<AudioClip>; 1],
}

impl MovementSounds {
	pub fn load(assets: &AssetServer) -> Self {
		Self {
			footstep: [assets.load(MovementClip::Footstep.path(0))],
			change_item: [assets.load(MovementClip::ChangeItem.path(0))],
		}
	}

	pub fn clip(&self, kind: MovementClip, variant: usize) -> &Handle<AudioClip> {
		let count = kind.variant_count();
		let index = if count == 0 { 0 } else { variant % count };
		match kind {
			MovementClip::Footstep => &self.footstep[index],
			MovementClip::ChangeItem => &self.change_item[index],
		}
	}

	pub fn play_footstep(
		&self,
		commands: &mut Commands,
		state: &mut MovementState,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		let variant = state.pick_foot(MovementClip::Footstep.variant_count());
		audio.play_or_queue(
			commands,
			self.clip(MovementClip::Footstep, variant),
			clips,
			SpatialOneShot::at(point)
				.radius(FOOTSTEP_SPATIAL_RADIUS)
				.gain(FOOTSTEP_VOLUME)
				.bus(AudioBus::Voices),
			mixer,
			listener,
			"character-footstep",
		);
	}

	pub fn play_change_item(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		audio.play_or_queue(
			commands,
			self.clip(MovementClip::ChangeItem, 0),
			clips,
			SpatialOneShot::at(point)
				.radius(CHANGE_ITEM_SPATIAL_RADIUS)
				.gain(CHANGE_ITEM_VOLUME)
				.bus(AudioBus::Voices),
			mixer,
			listener,
			"character-change-item",
		);
	}
}

pub(crate) fn setup_movement_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(MovementSounds::load(&assets));
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	fn asset_bytes(path: &str) -> Vec<u8> {
		let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets").join(path);
		match std::fs::read(&file) {
			Ok(bytes) => bytes,
			Err(_) => panic!("{}", file.display()),
		}
	}

	#[test]
	fn movement_paths_are_numbered_from_one() {
		assert_eq!(
			MovementClip::Footstep.path(0),
			"sound-effects/character/movement/footstep_001.wav"
		);
		assert_eq!(
			MovementClip::ChangeItem.path(0),
			"sound-effects/character/movement/change_item_001.wav"
		);
	}

	#[test]
	fn authored_movement_clips_are_mono() {
		for kind in MovementClip::VALUES {
			for variant in 0..kind.variant_count() {
				let path = kind.path(variant);
				assert!(
					crate::decode_wav_mono(&asset_bytes(&path)).is_ok(),
					"{path} must be authored mono"
				);
			}
		}
	}

	#[test]
	fn gait_emits_one_step_per_cycle() {
		let mut state = MovementState::seeded(3);
		assert_eq!(state.take_steps(2.0, 0.4), 0);
		assert_eq!(state.take_steps(2.0, 0.2), 1);
		assert!(state.phase < 1.0);
	}

	#[test]
	fn stopped_gait_resets() {
		let mut state = MovementState::seeded(1);
		state.phase = 0.8;
		assert_eq!(state.take_steps(0.0, 0.016), 0);
		assert_eq!(state.phase, 0.0);
	}
}
