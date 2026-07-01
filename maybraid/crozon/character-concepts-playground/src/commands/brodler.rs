//! `/brodler` commands for the Brodler concept species.

use bevy::prelude::*;
use clap::{Args, Subcommand};
use crozon_characters::species::{
	brodler::{
		assets::HornMesh,
		{BrodlerConfig, BrodlerEyeColor, BrodlerHeadMesh, BrodlerSkinColor},
	},
	common::{ClothingMesh, HairMesh},
};

use crate::{animation::ConceptAnimation, preview::ConceptPreviewConfig};

#[derive(Clone, Subcommand)]
pub enum Brodler {
	/// Spawn a Brodler through the resolved concepts pipeline.
	Preview(PreviewArgs),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PreviewArgs {
	#[arg(long, value_enum, default_value_t = BrodlerHeadMesh::Gaunt)]
	pub head: BrodlerHeadMesh,

	#[arg(long, value_enum, default_value_t = HornMesh::HarrowedCrown)]
	pub horns: HornMesh,

	#[arg(long, value_enum, default_value_t = HairMesh::None)]
	pub hair: HairMesh,

	/// Clothing layers to remap to the body rig. Repeat the flag for multiple layers.
	#[arg(long, value_enum)]
	pub clothing: Vec<ClothingMesh>,

	#[arg(long, value_enum, default_value_t = ConceptAnimation::Still)]
	pub animation: ConceptAnimation,

	#[arg(long, value_enum, default_value_t = BrodlerSkinColor::Red)]
	pub skin: BrodlerSkinColor,

	#[arg(long, value_enum, default_value_t = BrodlerEyeColor::Red)]
	pub eyes: BrodlerEyeColor,
}

impl Brodler {
	pub fn react(self, commands: &mut Commands) {
		match self {
			Self::Preview(args) => {
				let config = args.into_preview_config();
				commands.queue(move |world: &mut World| {
					*world.resource_mut::<ConceptPreviewConfig>() = config;
				});
			}
		}
	}
}

impl PreviewArgs {
	fn into_preview_config(self) -> ConceptPreviewConfig {
		let mut colors = crozon_characters::species::brodler::BrodlerColors::default();
		colors.skin = self.skin;
		colors.eyes = self.eyes;
		ConceptPreviewConfig::brodler_with_animation(
			BrodlerConfig { head: self.head, horns: self.horns, hair: self.hair, clothing: self.clothing, colors },
			self.animation,
		)
	}
}
