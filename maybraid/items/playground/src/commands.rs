//! In-game clap commands for the items playground.

use bevy::prelude::*;
use clap::{Args, Parser};
use firearms::{
	BarrelMesh, BodyMesh, FirearmConcept, FirearmKit, FirearmPose, GripMesh, KitBone, StockMesh,
	TriggerBoxMesh,
};
use game_commands::command::{CommandScript, GameCommand};

use crate::preview::PreviewConfig;

pub const PLAYGROUND_CLI_NAME: &str = "items";
pub type Script = CommandScript<PlaygroundCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "items",
	version,
	about = "Items playground commands (in-game after `/` or process argv)",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum PlaygroundCommand {
	Help,
	Script(Script),
	/// Spawn a named firearm concept (body plus that kit's optional parts).
	Show {
		concept: FirearmConcept,
	},
	/// Set individual kit slots. Omitted flags keep the current kit; `none` clears an optional slot.
	Kit(KitArgs),
	/// Set length (bone Y) and/or thickness (bone XZ) on a kit bone. Omitted flags keep the current value.
	Scale(ScaleArgs),
}

#[derive(Clone, Copy, Args, Debug, Default, PartialEq, Eq)]
#[command(rename_all = "kebab-case")]
pub struct KitArgs {
	/// Required slot. Always a body mesh (no `none`).
	#[arg(long, value_enum)]
	pub body: Option<BodyMesh>,
	#[arg(long, value_enum)]
	pub barrel: Option<BarrelMesh>,
	#[arg(long, value_enum)]
	pub trigger_box: Option<TriggerBoxMesh>,
	#[arg(long, value_enum)]
	pub grip: Option<GripMesh>,
	#[arg(long, value_enum)]
	pub stock: Option<StockMesh>,
}

impl KitArgs {
	fn apply(self, kit: &mut FirearmKit) {
		if let Some(body) = self.body {
			kit.body = body;
		}
		if let Some(barrel) = self.barrel {
			kit.barrel = barrel;
		}
		if let Some(trigger_box) = self.trigger_box {
			kit.trigger_box = trigger_box;
		}
		if let Some(grip) = self.grip {
			kit.grip = grip;
		}
		if let Some(stock) = self.stock {
			kit.stock = stock;
		}
	}

	fn summary(&self) -> String {
		let mut parts = Vec::new();
		if let Some(body) = self.body {
			parts.push(format!("--body {}", body.label()));
		}
		if let Some(barrel) = self.barrel {
			parts.push(format!("--barrel {}", barrel.label()));
		}
		if let Some(trigger_box) = self.trigger_box {
			parts.push(format!("--trigger-box {}", trigger_box.label()));
		}
		if let Some(grip) = self.grip {
			parts.push(format!("--grip {}", grip.label()));
		}
		if let Some(stock) = self.stock {
			parts.push(format!("--stock {}", stock.label()));
		}
		if parts.is_empty() {
			"kit (unchanged)".into()
		} else {
			format!("kit {}", parts.join(" "))
		}
	}
}

#[derive(Clone, Copy, Args, Debug, Default, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct ScaleArgs {
	/// Kit bone: `body`, `barrel`, `trigger-box`, `grip`, `stock`.
	#[arg(value_enum)]
	pub bone: KitBone,
	#[arg(long)]
	pub length: Option<f32>,
	#[arg(long)]
	pub thickness: Option<f32>,
}

impl ScaleArgs {
	fn apply(self, pose: &mut FirearmPose) {
		let fit = pose.fit_mut(self.bone);
		if let Some(length) = self.length {
			fit.length = length;
		}
		if let Some(thickness) = self.thickness {
			fit.thickness = thickness;
		}
	}

	fn summary(&self) -> String {
		let mut parts = vec![self.bone.label().to_string()];
		if let Some(length) = self.length {
			parts.push(format!("--length {length}"));
		}
		if let Some(thickness) = self.thickness {
			parts.push(format!("--thickness {thickness}"));
		}
		if self.length.is_none() && self.thickness.is_none() {
			format!("scale {} (unchanged)", self.bone.label())
		} else {
			format!("scale {}", parts.join(" "))
		}
	}
}

impl PlaygroundCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Help => *console = Self::long_help_string(),
			Self::Script(script) => script.run(commands, console),
			Self::Show { concept } => {
				let kit = concept.kit();
				*console = format!("show {}", concept.label());
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<PreviewConfig>() =
						PreviewConfig { kit, pose: FirearmPose::default() };
				});
			}
			Self::Kit(args) => {
				*console = args.summary();
				commands.queue(move |world: &mut World| {
					args.apply(&mut world.resource_mut::<PreviewConfig>().kit);
				});
			}
			Self::Scale(args) => {
				*console = args.summary();
				commands.queue(move |world: &mut World| {
					args.apply(&mut world.resource_mut::<PreviewConfig>().pose);
				});
			}
		}
	}
}

impl GameCommand for PlaygroundCommand {
	const CLI_NAME: &'static str = PLAYGROUND_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_show_bullpup() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show bullpup")?;
		let PlaygroundCommand::Show { concept } = command else {
			return Err("expected show".into());
		};
		assert_eq!(concept, FirearmConcept::Bullpup);
		Ok(())
	}

	#[test]
	fn parses_kit_slot_flags() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line(
			"kit --body silopup --barrel laznard --grip none --trigger-box paddle",
		)?;
		let PlaygroundCommand::Kit(args) = command else {
			return Err("expected kit".into());
		};
		assert_eq!(args.body, Some(BodyMesh::Silopup));
		assert_eq!(args.barrel, Some(BarrelMesh::Laznard));
		assert_eq!(args.grip, Some(GripMesh::None));
		assert_eq!(args.trigger_box, Some(TriggerBoxMesh::Paddle));
		assert!(args.stock.is_none());
		Ok(())
	}

	#[test]
	fn kit_patch_keeps_omitted_slots() {
		let mut kit = FirearmConcept::Bullpup.kit();
		KitArgs {
			barrel: Some(BarrelMesh::Laznard),
			grip: Some(GripMesh::None),
			..KitArgs::default()
		}
		.apply(&mut kit);
		assert_eq!(kit.body, BodyMesh::Bullpup);
		assert_eq!(kit.barrel, BarrelMesh::Laznard);
		assert_eq!(kit.grip, GripMesh::None);
	}

	#[test]
	fn parses_scale_length_and_thickness() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line(
			"scale barrel --length 1.5 --thickness 0.8",
		)?;
		let PlaygroundCommand::Scale(args) = command else {
			return Err("expected scale".into());
		};
		assert_eq!(args.bone, KitBone::Barrel);
		assert_eq!(args.length, Some(1.5));
		assert_eq!(args.thickness, Some(0.8));
		let mut pose = FirearmPose::default();
		args.apply(&mut pose);
		assert_eq!(pose.barrel.length, 1.5);
		assert_eq!(pose.barrel.thickness, 0.8);
		assert_eq!(pose.body.length, 1.0);
		Ok(())
	}
}
