//! `/character` subcommands for modular rig assembly.

use bevy::prelude::*;
use clap::{Args, Subcommand};

use crate::animation::AnimationMode;
use crate::character::{request_dump_bones, CharacterConfig};

#[derive(Clone, Subcommand)]
pub enum Character {
	/// Spawn a rig and optional modular skinned parts.
	Assemble(AssembleArgs),
	/// Print the live rig bone hierarchy to the HUD console.
	DumpBones,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct AssembleArgs {
	/// Shared armature GLB path (under maybraid/assets).
	#[arg(long, default_value = crate::character::DEFAULT_RIG)]
	pub rig: String,

	/// Body mesh GLB path.
	#[arg(long)]
	pub body: Option<String>,

	/// Head mesh GLB path.
	#[arg(long)]
	pub head: Option<String>,

	/// Mouth mesh GLB path.
	#[arg(long)]
	pub mouth: Option<String>,

	/// Nose mesh GLB path.
	#[arg(long)]
	pub nose: Option<String>,

	/// Procedural animation applied to the rig (`run` or `squat`).
	#[arg(long, value_enum, default_value_t = AnimationMode::Run)]
	pub animation: AnimationMode,

	/// Translation `x,y,z` in world units.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub translate: Vec3,

	/// Scale factors `x,y,z`.
	#[arg(long, default_value = "1,1,1", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub scale: Vec3,

	/// Euler rotation in degrees around X, then Y, then Z.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub rotate_euler: Vec3,
}

impl Character {
	pub fn react(self, commands: &mut Commands) {
		match self {
			Character::Assemble(args) => {
				let config = args.into_character_config();
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<CharacterConfig>() = config;
				});
			}
			Character::DumpBones => request_dump_bones(commands),
		}
	}
}

impl AssembleArgs {
	fn into_character_config(self) -> CharacterConfig {
		let rot = Quat::from_euler(
			EulerRot::XYZ,
			self.rotate_euler.x.to_radians(),
			self.rotate_euler.y.to_radians(),
			self.rotate_euler.z.to_radians(),
		);
		CharacterConfig {
			rig: self.rig,
			body: self.body,
			head: self.head,
			mouth: self.mouth,
			nose: self.nose,
			animation: self.animation,
			transform: Transform::from_translation(self.translate)
				.with_rotation(rot)
				.with_scale(self.scale),
		}
	}
}

fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).collect();
	if parts.len() != 3 {
		return Err(format!("expected x,y,z, got {s:?}"));
	}
	let x = parts[0].parse::<f32>().map_err(|e| e.to_string())?;
	let y = parts[1].parse::<f32>().map_err(|e| e.to_string())?;
	let z = parts[2].parse::<f32>().map_err(|e| e.to_string())?;
	Ok(Vec3::new(x, y, z))
}
