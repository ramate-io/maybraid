//! Bus gains and short-lived ducks. Applied each frame to live voices.

use bevy::prelude::*;

const BUS_COUNT: usize = 6;

/// Mix groups. [`AudioBus::Master`] scales every other bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AudioBus {
	Master = 0,
	PlayerWeapon = 1,
	Weapons = 2,
	Impacts = 3,
	Ambience = 4,
	Ui = 5,
}

impl AudioBus {
	pub const ALL: [Self; BUS_COUNT] =
		[Self::Master, Self::PlayerWeapon, Self::Weapons, Self::Impacts, Self::Ambience, Self::Ui];

	fn index(self) -> usize {
		self as usize
	}
}

#[derive(Clone, Copy, Debug)]
struct Duck {
	bus: AudioBus,
	linear: f32,
	elapsed: f32,
	duration: f32,
}

/// Default bus levels plus transient ducks.
#[derive(Resource, Debug)]
pub struct Mixer {
	bus: [f32; BUS_COUNT],
	ducks: Vec<Duck>,
}

impl Default for Mixer {
	fn default() -> Self {
		let mut bus = [1.0; BUS_COUNT];
		bus[AudioBus::Weapons.index()] = 0.8;
		Self { bus, ducks: Vec::new() }
	}
}

impl Mixer {
	pub fn set_bus(&mut self, bus: AudioBus, linear: f32) {
		self.bus[bus.index()] = linear.max(0.0);
	}

	/// Multiply `bus` by `db` for `seconds`, then release.
	pub fn duck(&mut self, bus: AudioBus, db: f32, seconds: f32) {
		if seconds <= 0.0 {
			return;
		}
		self.ducks
			.push(Duck { bus, linear: db_to_linear(db), elapsed: 0.0, duration: seconds });
	}

	/// Player-weapon transient: pull ambience and other weapons down briefly.
	pub fn duck_player_shot(&mut self) {
		self.duck(AudioBus::Ambience, -4.0, 0.12);
		self.duck(AudioBus::Weapons, -2.0, 0.08);
	}

	pub fn tick(&mut self, dt: f32) {
		for duck in &mut self.ducks {
			duck.elapsed += dt;
		}
		self.ducks.retain(|duck| duck.elapsed < duck.duration);
	}

	/// Master × bus × active ducks on that bus.
	pub fn gain(&self, bus: AudioBus) -> f32 {
		let mut gain = self.bus[AudioBus::Master.index()] * self.bus[bus.index()];
		for duck in &self.ducks {
			if duck.bus == bus {
				gain *= duck.linear;
			}
		}
		gain
	}
}

pub(crate) fn tick_mixer(time: Res<Time>, mut mixer: ResMut<Mixer>) {
	mixer.tick(time.delta_secs());
}

pub(crate) fn db_to_linear(db: f32) -> f32 {
	10.0_f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn player_shot_ducks_ambience_not_player_weapon() {
		let mut mixer = Mixer::default();
		let before_ambience = mixer.gain(AudioBus::Ambience);
		let before_player = mixer.gain(AudioBus::PlayerWeapon);
		mixer.duck_player_shot();
		assert!(mixer.gain(AudioBus::Ambience) < before_ambience);
		assert!((mixer.gain(AudioBus::PlayerWeapon) - before_player).abs() < 1e-5);
		mixer.tick(1.0);
		assert!((mixer.gain(AudioBus::Ambience) - before_ambience).abs() < 1e-5);
	}

	#[test]
	fn world_weapons_default_quieter_than_player() {
		let mixer = Mixer::default();
		assert!(mixer.gain(AudioBus::Weapons) < mixer.gain(AudioBus::PlayerWeapon));
	}

	#[test]
	fn minus_six_db_is_about_half() {
		let linear = db_to_linear(-6.0);
		assert!((linear - 0.501).abs() < 0.02);
	}

	#[test]
	fn all_buses_are_indexed() {
		assert_eq!(AudioBus::ALL.len(), BUS_COUNT);
	}
}
