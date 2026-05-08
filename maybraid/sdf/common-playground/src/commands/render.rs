use bevy::prelude::*;
use clap::{Args, Parser};
use sdf_common::TaperedCylinder;

#[derive(Debug, Clone, Parser)]
pub struct RenderHelper<T: Args> {
	#[clap(flatten)]
	pub inner: T,

	#[clap(long, default_value = "4")]
	pub res_2: u8,

	#[clap(long, default_value = "1.0")]
	pub x_scale: f32,

	#[clap(long, default_value = "1.0")]
	pub y_scale: f32,

	#[clap(long, default_value = "1.0")]
	pub z_scale: f32,

	#[clap(long, default_value = "0.0")]
	pub x: f32,

	#[clap(long, default_value = "0.0")]
	pub y: f32,

	#[clap(long, default_value = "0.0")]
	pub z: f32,
}

#[derive(Debug, Clone, Parser)]
pub enum Render {
	/// Render a tapered cylinder.
	TaperedCylinder(RenderHelper<TaperedCylinder>),
}

impl RenderHelper<TaperedCylinder> {
	pub fn react(&self, _commands: &mut Commands) {
		// ...
	}
}

impl Render {
	pub fn react(&self, commands: &mut Commands) {
		match self {
			Self::TaperedCylinder(helper) => helper.react(commands),
		}
	}
}
