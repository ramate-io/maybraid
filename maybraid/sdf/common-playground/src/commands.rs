pub mod render;
use bevy::prelude::*;
use clap::Parser;
pub use render::Render;

#[derive(Debug, Clone, Parser)]
pub enum Command {
	#[clap(subcommand)]
	Render(Render),
}

impl Command {
	pub fn react(&self, commands: &mut Commands) {
		match self {
			Self::Render(render) => render.react(commands),
		}
	}
}
