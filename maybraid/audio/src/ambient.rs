//! Character ambient foley. Numbered variants live under
//! `sound-effects/character/ambient/`.

use bevy::prelude::*;

use crate::asset::AudioClip;
use crate::backend::Audio;
use crate::flinch::pick_variant;
use crate::mixer::{AudioBus, Mixer};
use crate::spatial::SpatialOneShot;

pub const BIRDSONG_SPATIAL_SCALE: f32 = 0.02;
pub const BIRDSONG_SPATIAL_RADIUS: f32 = 1.0 / BIRDSONG_SPATIAL_SCALE;
pub const BIRDSONG_VOLUME: f32 = 0.5;
pub const HERD_GRUNT_SPATIAL_SCALE: f32 = 0.05;
pub const HERD_GRUNT_SPATIAL_RADIUS: f32 = 1.0 / HERD_GRUNT_SPATIAL_SCALE;
pub const HERD_GRUNT_VOLUME: f32 = 1.1;
pub const HERD_WAIL_SPATIAL_SCALE: f32 = 0.04;
pub const HERD_WAIL_SPATIAL_RADIUS: f32 = 1.0 / HERD_WAIL_SPATIAL_SCALE;
pub const HERD_WAIL_VOLUME: f32 = 1.5;

/// Authored ambient family. Variants are `{slug}_{index:03}.wav`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AmbientClip {
	Birdsong,
	HerdGrunt,
	HerdWail,
}

impl AmbientClip {
	pub const VALUES: [Self; 3] = [Self::Birdsong, Self::HerdGrunt, Self::HerdWail];

	pub const fn slug(self) -> &'static str {
		match self {
			Self::Birdsong => "birdsong",
			Self::HerdGrunt => "herd_grunt",
			Self::HerdWail => "herd_wail",
		}
	}

	pub const fn variant_count(self) -> usize {
		match self {
			Self::Birdsong => 2,
			Self::HerdGrunt => 3,
			Self::HerdWail => 1,
		}
	}

	pub fn path(self, variant: usize) -> String {
		let index = variant % self.variant_count() + 1;
		format!("sound-effects/character/ambient/{}_{index:03}.wav", self.slug())
	}
}

/// Skip-last pick state for ambient families.
#[derive(Clone, Copy, Debug, Default)]
pub struct AmbientPick {
	pub last_bird: Option<u8>,
	pub last_grunt: Option<u8>,
}

impl AmbientPick {
	pub fn bird(&mut self, noise: &mut u64) -> usize {
		let variant = pick_variant(
			AmbientClip::Birdsong.variant_count(),
			self.last_bird.map(usize::from),
			noise,
		);
		self.last_bird = Some(variant as u8);
		variant
	}

	pub fn grunt(&mut self, noise: &mut u64) -> usize {
		let variant = pick_variant(
			AmbientClip::HerdGrunt.variant_count(),
			self.last_grunt.map(usize::from),
			noise,
		);
		self.last_grunt = Some(variant as u8);
		variant
	}
}

/// Loaded ambient clips.
#[derive(Resource)]
pub struct AmbientSounds {
	birdsong: [Handle<AudioClip>; 2],
	herd_grunt: [Handle<AudioClip>; 3],
	herd_wail: [Handle<AudioClip>; 1],
}

impl AmbientSounds {
	pub fn load(assets: &AssetServer) -> Self {
		Self {
			birdsong: [
				assets.load(AmbientClip::Birdsong.path(0)),
				assets.load(AmbientClip::Birdsong.path(1)),
			],
			herd_grunt: [
				assets.load(AmbientClip::HerdGrunt.path(0)),
				assets.load(AmbientClip::HerdGrunt.path(1)),
				assets.load(AmbientClip::HerdGrunt.path(2)),
			],
			herd_wail: [assets.load(AmbientClip::HerdWail.path(0))],
		}
	}

	pub fn clip(&self, kind: AmbientClip, variant: usize) -> &Handle<AudioClip> {
		let count = kind.variant_count();
		let index = if count == 0 { 0 } else { variant % count };
		match kind {
			AmbientClip::Birdsong => &self.birdsong[index],
			AmbientClip::HerdGrunt => &self.herd_grunt[index],
			AmbientClip::HerdWail => &self.herd_wail[index],
		}
	}

	pub fn play_birdsong(
		&self,
		commands: &mut Commands,
		pick: &mut AmbientPick,
		noise: &mut u64,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		self.play(
			commands,
			AmbientClip::Birdsong,
			pick.bird(noise),
			clips,
			audio,
			mixer,
			listener,
			point,
			BIRDSONG_SPATIAL_RADIUS,
			BIRDSONG_VOLUME,
			AudioBus::Ambience,
			"ambient-birdsong",
		);
	}

	pub fn play_herd_grunt(
		&self,
		commands: &mut Commands,
		pick: &mut AmbientPick,
		noise: &mut u64,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		self.play(
			commands,
			AmbientClip::HerdGrunt,
			pick.grunt(noise),
			clips,
			audio,
			mixer,
			listener,
			point,
			HERD_GRUNT_SPATIAL_RADIUS,
			HERD_GRUNT_VOLUME,
			AudioBus::Voices,
			"ambient-herd-grunt",
		);
	}

	pub fn play_herd_wail(
		&self,
		commands: &mut Commands,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
	) {
		self.play(
			commands,
			AmbientClip::HerdWail,
			0,
			clips,
			audio,
			mixer,
			listener,
			point,
			HERD_WAIL_SPATIAL_RADIUS,
			HERD_WAIL_VOLUME,
			AudioBus::Voices,
			"ambient-herd-wail",
		);
	}

	fn play(
		&self,
		commands: &mut Commands,
		kind: AmbientClip,
		variant: usize,
		clips: &Assets<AudioClip>,
		audio: &Audio,
		mixer: &mut Mixer,
		listener: &GlobalTransform,
		point: Vec3,
		radius: f32,
		gain: f32,
		bus: AudioBus,
		name: &'static str,
	) {
		audio.play_or_queue(
			commands,
			self.clip(kind, variant),
			clips,
			SpatialOneShot::at(point).radius(radius).gain(gain).bus(bus),
			mixer,
			listener,
			name,
		);
	}
}

pub(crate) fn setup_ambient_sounds(mut commands: Commands, assets: Res<AssetServer>) {
	commands.insert_resource(AmbientSounds::load(&assets));
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
	fn ambient_paths_are_numbered_from_one() {
		assert_eq!(
			AmbientClip::Birdsong.path(0),
			"sound-effects/character/ambient/birdsong_001.wav"
		);
		assert_eq!(
			AmbientClip::Birdsong.path(1),
			"sound-effects/character/ambient/birdsong_002.wav"
		);
		assert_eq!(
			AmbientClip::HerdGrunt.path(2),
			"sound-effects/character/ambient/herd_grunt_003.wav"
		);
		assert_eq!(
			AmbientClip::HerdWail.path(0),
			"sound-effects/character/ambient/herd_wail_001.wav"
		);
	}

	#[test]
	fn authored_ambient_clips_are_mono() {
		for kind in AmbientClip::VALUES {
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
	fn birdsong_pick_skips_the_last_variant() {
		let mut pick = AmbientPick::default();
		let mut noise = 3_u64;
		pick.bird(&mut noise);
		for _ in 0..24 {
			let last = pick.last_bird.unwrap() as usize;
			let next = pick.bird(&mut noise);
			assert_ne!(next, last);
			assert!(next < 2);
		}
	}
}
