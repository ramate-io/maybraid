//! Runtime plugin: stamp CharacterRig / CharacterPart on nested LodScene hosts.

use bevy::prelude::*;
use lod::LodRefreshSystems;

use crate::nodes::{PartNode, RigNode};
use crate::rig::{
	ActiveRigPose, BoneMap, CharacterPart, CharacterRig, LodCharacterRig, RigBindScales,
};
use crate::socket::{SkinRefRoot, SocketRefRoot};

/// Nested character hosts are prepared after chunk fulfill spawns them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum CharacterHostSystems {
	Prepare,
}

/// Installs character LodScene host preparation (socket/skin fulfill is scheduled by the app).
pub struct CharacterComponentsPlugin;

impl Plugin for CharacterComponentsPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(Update, CharacterHostSystems::Prepare.after(LodRefreshSystems::Fulfill))
			.add_systems(
				Update,
				(prepare_rig_hosts, prepare_part_hosts).in_set(CharacterHostSystems::Prepare),
			);
	}
}

fn prepare_rig_hosts(mut commands: Commands, added: Query<(Entity, &RigNode), Added<RigNode>>) {
	for (entity, node) in &added {
		let mut entity = commands.entity(entity);
		entity.insert((
			CharacterRig { role: node.id.role(), skeleton: node.skeleton },
			LodCharacterRig,
			BoneMap::default(),
			ActiveRigPose { pose: node.pose.clone() },
			RigBindScales::default(),
			node.normalization.transform(),
		));
		if let Some(socket) = node.socket {
			entity.insert(SocketRefRoot(socket));
		}
	}
}

fn prepare_part_hosts(mut commands: Commands, added: Query<(Entity, &PartNode), Added<PartNode>>) {
	for (entity, node) in &added {
		let mut entity = commands.entity(entity);
		entity.insert((CharacterPart { slot: node.slot }, node.authored_transform()));
		if let Some(socket) = node.socket {
			entity.insert(SocketRefRoot(socket));
		}
		if let Some(skin) = node.skin {
			entity.insert(SkinRefRoot(skin));
		}
	}
}
