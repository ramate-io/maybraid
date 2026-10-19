use bevy::prelude::*;
use character_rigs::Side;
use characters::{
	spawn_fixed_character_assembly, species::braidman::BraidmanConfig,
	species::epiphant::EpiphantConfig, AnimProgress, AnimRefRoot, CharacterMembers,
	CharacterRecipe, CharacterRig, CharacterRigRole, CharacterRoot, EyeGlowControl,
	GOD_BEAM_EYE_GLOW_INTENSITY,
};
use clap::ValueEnum;

use crate::animation::{AnimationMode, AnimationPlayback};

/// Lab subject. The playground is a clip viewer, not a species creator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum CharacterSpecies {
	#[default]
	Braidman,
	Epiphant,
}

impl CharacterSpecies {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Braidman => "braidman",
			Self::Epiphant => "epiphant",
		}
	}
}

#[derive(Component)]
pub struct PlaygroundCharacter;

#[derive(Resource, Clone)]
pub struct CharacterConfig {
	pub species: CharacterSpecies,
	pub animation: AnimationMode,
	/// Sided gestures (`jab`, `think-again`) read this. Ignored by symmetric clips.
	pub side: Side,
	pub transform: Transform,
}

impl Default for CharacterConfig {
	fn default() -> Self {
		Self {
			species: CharacterSpecies::Braidman,
			animation: AnimationMode::default(),
			side: Side::Right,
			transform: Transform::IDENTITY,
		}
	}
}

impl CharacterConfig {
	pub fn status_label(&self) -> String {
		let mut parts = vec![format!("species={}", self.species.label())];
		parts.push(format!("animation={}", self.animation.label()));
		if self.animation.uses_side() {
			parts.push(format!(
				"side={}",
				match self.side {
					Side::Left => "left",
					Side::Right => "right",
				}
			));
		}
		format!("character {}", parts.join(" "))
	}

	pub fn spawn_key(&self) -> String {
		format!("{:?}|{:?}|{:?}", self.species, self.transform.translation, self.transform.rotation,)
	}

	pub fn anim_ref(&self) -> characters::AnimRef {
		self.animation.anim_ref(self.side)
	}
}

#[derive(Resource, Default)]
pub(crate) struct CharacterSyncState {
	spawn_key: String,
}

pub(crate) fn sync_character(
	mut commands: Commands,
	config: Res<CharacterConfig>,
	mut sync_state: ResMut<CharacterSyncState>,
	roots: Query<Entity, With<PlaygroundCharacter>>,
) {
	let spawn_key = config.spawn_key();
	if sync_state.spawn_key == spawn_key {
		return;
	}
	sync_state.spawn_key = spawn_key;

	for entity in &roots {
		commands.entity(entity).try_despawn();
	}

	let entity = match config.species {
		CharacterSpecies::Braidman => spawn_fixed_character_assembly(
			&mut commands,
			&CharacterRecipe::clothed(&BraidmanConfig::default_preview()),
			config.transform,
		),
		CharacterSpecies::Epiphant => spawn_fixed_character_assembly(
			&mut commands,
			&CharacterRecipe::clothed(&EpiphantConfig::default_preview()),
			config.transform,
		),
	};
	commands.entity(entity).insert(PlaygroundCharacter);
	if let Some(control) = eye_glow_from_env() {
		commands.entity(entity).insert(control);
	}
}

/// `MAYBRAID_EYE_GLOW_DEMO=off|mid|on` — preview capture helper (optional).
fn eye_glow_from_env() -> Option<EyeGlowControl> {
	match std::env::var("MAYBRAID_EYE_GLOW_DEMO").as_deref() {
		Ok("off") => Some(EyeGlowControl::off()),
		Ok("mid") => {
			let mut control = EyeGlowControl::god_beam_warning();
			control.intensity = 0.35;
			control.target_intensity = 0.35;
			control.transition_secs = 0.0;
			Some(control)
		},
		Ok("on") | Ok("full") => {
			let mut control = EyeGlowControl::god_beam_warning();
			control.intensity = GOD_BEAM_EYE_GLOW_INTENSITY;
			control.target_intensity = GOD_BEAM_EYE_GLOW_INTENSITY;
			control.transition_secs = 0.0;
			Some(control)
		},
		_ => None,
	}
}

/// Write the session clip onto the body member (`AnimRefRoot` defaults to still).
pub(crate) fn stamp_anim(
	mut commands: Commands,
	config: Res<CharacterConfig>,
	roots: Query<&CharacterMembers, With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	let desired = config.anim_ref();
	for members in &roots {
		for member in members.iter() {
			if !rigs.get(member).is_ok_and(|rig| rig.role == CharacterRigRole::Body) {
				continue;
			}
			let needs = match anims.get(member) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs {
				commands.entity(member).insert(AnimRefRoot(desired));
			}
		}
	}
}

/// Own mailbox time so `/character playback` can pause, scrub, and `--once`.
pub(crate) fn drive_playback(
	mut commands: Commands,
	time: Res<Time>,
	config: Res<CharacterConfig>,
	mut playback: ResMut<AnimationPlayback>,
	roots: Query<&CharacterMembers, With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
) {
	playback.advance(
		time.delta_secs(),
		config.animation.clip_duration(),
		config.animation.clip_speed(),
	);
	let progress = config
		.animation
		.mailbox_progress(playback.elapsed + playback.phase, playback.looping);
	for members in &roots {
		for member in members.iter() {
			if rigs.get(member).is_ok_and(|rig| rig.role == CharacterRigRole::Body) {
				commands.entity(member).insert(AnimProgress(progress));
			}
		}
	}
}

pub fn request_dump_bones(commands: &mut Commands) {
	commands.queue(|world: &mut World| {
		world.resource_mut::<crate::skinning::DumpBonesRequest>().0 = true;
	});
}
