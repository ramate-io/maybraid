use crate::tree::meshes::trunk::segment::SimpleTrunkSegment;
use bevy::prelude::*;

#[derive(Component, Clone)]
pub enum TreeStick {
	SimpleTrunkSegment(SimpleTrunkSegment),
}
