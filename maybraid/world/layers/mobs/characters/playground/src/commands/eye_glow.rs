//! `/character eye-glow` — preview per-character eye warning glow.

use bevy::prelude::*;
use characters::{
	EyeGlowControl, GOD_BEAM_EYE_GLOW_COLOR, GOD_BEAM_EYE_GLOW_EMISSIVE_GAIN,
};
use clap::{Args, Subcommand};

use crate::character::PlaygroundCharacter;

#[derive(Clone, Subcommand)]
pub enum EyeGlow {
	/// Fade in the God Beam warning preset.
	On(EyeGlowOnArgs),
	/// Fade glow off.
	Off,
	/// Snap glow off immediately (interrupt any fade).
	Reset,
	/// Set a custom intensity (0–1) with optional transition seconds.
	Set(EyeGlowSetArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct EyeGlowOnArgs {
	#[arg(long, default_value_t = 0.35)]
	pub transition_secs: f32,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct EyeGlowSetArgs {
	#[arg(long, default_value_t = 0.5)]
	pub intensity: f32,
	#[arg(long, default_value_t = 0.35)]
	pub transition_secs: f32,
}

impl EyeGlow {
	pub fn react(self, commands: &mut Commands) {
		match self {
			EyeGlow::On(args) => {
				let transition_secs = args.transition_secs.max(0.0);
				commands.queue(move |world: &mut World| {
					apply_to_playground_characters(world, |control| {
						control.start_god_beam_warning();
						control.transition_secs = transition_secs;
					});
				});
			}
			EyeGlow::Off => {
				commands.queue(|world: &mut World| {
					apply_to_playground_characters(world, |control| control.cancel());
				});
			}
			EyeGlow::Reset => {
				commands.queue(|world: &mut World| {
					apply_to_playground_characters(world, |control| control.reset());
				});
			}
			EyeGlow::Set(args) => {
				let intensity = args.intensity.clamp(0.0, 1.0);
				let transition_secs = args.transition_secs.max(0.0);
				commands.queue(move |world: &mut World| {
					apply_to_playground_characters(world, |control| {
						control.start_warning(
							GOD_BEAM_EYE_GLOW_COLOR,
							intensity,
							Some(transition_secs),
						);
						control.emissive_gain = GOD_BEAM_EYE_GLOW_EMISSIVE_GAIN;
					});
				});
			}
		}
	}
}

fn apply_to_playground_characters(world: &mut World, mutator: impl Fn(&mut EyeGlowControl)) {
	let roots: Vec<Entity> = world
		.query_filtered::<Entity, With<PlaygroundCharacter>>()
		.iter(world)
		.collect();
	for root in roots {
		if world.get::<EyeGlowControl>(root).is_none() {
			world.entity_mut(root).insert(EyeGlowControl::off());
		}
		if let Some(mut control) = world.get_mut::<EyeGlowControl>(root) {
			mutator(&mut control);
		}
	}
}
