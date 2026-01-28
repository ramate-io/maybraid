use crate::tree::meshes::canopy::ball::NoisyBall;
use bevy::prelude::*;

#[derive(Component, Clone)]
pub enum TreeBall {
	NoisyBall(NoisyBall),
}
