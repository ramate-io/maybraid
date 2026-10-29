//! Skin-remap fulfill for [`crate::socket::SkinRef`] and GLTF duplicate-armature prune.

use std::collections::{HashMap, HashSet};

use bevy::{
	asset::AssetId,
	ecs::message::{MessageCursor, Messages},
	mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
	prelude::*,
	world_serialization::{WorldInstance, WorldInstanceSpawner},
};

use crate::assembly::CharacterPartSlot;
use crate::member::{find_member_rig, CharacterMembers, MemberOf};
use crate::rig::{
	BoneMap, CharacterPart, CharacterRig, NeedsDuplicateScenePrune, NeedsSkinRemap,
	NoMatchingArmature, PartRigRef,
};
use crate::socket::{SkinRefApplied, SkinRefRoot};
use character_rigs::rigs::humanoid_v0::HumanoidForearmEndpoints;

mod forearm_endpoints;
use forearm_endpoints::forearm_terminal_points;

/// Per-system binding observations used to invalidate body terminal calibration.
#[derive(Default)]
pub struct BodyEndpointBindings {
	parts: HashMap<Entity, BodyEndpointBinding>,
	mesh_events: MessageCursor<AssetEvent<Mesh>>,
	bind_events: MessageCursor<AssetEvent<SkinnedMeshInverseBindposes>>,
}

struct BodyEndpointBinding {
	rig: Entity,
	skins: Vec<BodySkinBinding>,
}

struct BodySkinBinding {
	entity: Entity,
	mesh: Option<AssetId<Mesh>>,
	inverse_bindposes: AssetId<SkinnedMeshInverseBindposes>,
}

/// Drop [`SkinRefApplied`] when the identity changes so fulfill re-resolves.
pub fn invalidate_changed_skin_ref_roots(
	mut commands: Commands,
	changed: Query<Entity, (Changed<SkinRefRoot>, With<SkinRefApplied>)>,
) {
	for entity in &changed {
		commands.entity(entity).remove::<SkinRefApplied>();
	}
}

pub fn fulfill_skin_ref_roots(
	mut commands: Commands,
	pending: Query<(Entity, &SkinRefRoot), (With<CharacterPart>, Without<SkinRefApplied>)>,
	member_of: Query<&MemberOf>,
	members: Query<&CharacterMembers>,
	rigs: Query<(Entity, &CharacterRig, &BoneMap)>,
) {
	for (entity, SkinRefRoot(skin)) in &pending {
		let Ok(MemberOf(root)) = member_of.get(entity) else {
			continue;
		};
		let Ok(character_members) = members.get(*root) else {
			continue;
		};
		let Some((rig_root, map)) = find_member_rig(character_members, skin.target.role(), &rigs)
		else {
			continue;
		};
		if map.by_name.is_empty() {
			continue;
		}
		commands
			.entity(entity)
			.insert((PartRigRef { rig_root }, NeedsSkinRemap, SkinRefApplied));
	}
}

pub fn remap_part_skin_to_rig(
	mut commands: Commands,
	part_roots: Query<
		(Entity, &Children, &CharacterPart, &PartRigRef, &NeedsSkinRemap),
		(With<CharacterPart>, Without<NoMatchingArmature>),
	>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	mut skinned_meshes: Query<(&mut SkinnedMesh, Option<&Mesh3d>)>,
	geometry: (
		Option<Res<Assets<Mesh>>>,
		Option<Res<Assets<SkinnedMeshInverseBindposes>>>,
		Option<Res<Messages<AssetEvent<Mesh>>>>,
		Option<Res<Messages<AssetEvent<SkinnedMeshInverseBindposes>>>>,
	),
	all_parts: Query<&PartRigRef, With<CharacterPart>>,
	endpoint_components: Query<&HumanoidForearmEndpoints>,
	mut endpoint_bindings: Local<BodyEndpointBindings>,
	scene_instances: Query<&WorldInstance>,
	scene_spawner: Res<WorldInstanceSpawner>,
) {
	let (mesh_assets, bind_assets, mesh_events, bind_events) = geometry;
	let tracker = &mut *endpoint_bindings;
	let changed_meshes: HashSet<_> = mesh_events
		.as_deref()
		.map(|events| {
			tracker
				.mesh_events
				.read(events)
				.filter_map(|event| match event {
					AssetEvent::Added { id }
					| AssetEvent::Modified { id }
					| AssetEvent::Removed { id } => Some(*id),
					_ => None,
				})
				.collect()
		})
		.unwrap_or_default();
	let changed_binds: HashSet<_> = bind_events
		.as_deref()
		.map(|events| {
			tracker
				.bind_events
				.read(events)
				.filter_map(|event| match event {
					AssetEvent::Added { id }
					| AssetEvent::Modified { id }
					| AssetEvent::Removed { id } => Some(*id),
					_ => None,
				})
				.collect()
		})
		.unwrap_or_default();
	let invalid: Vec<_> = tracker
		.parts
		.iter()
		.filter_map(|(part, binding)| {
			let rebound =
				all_parts.get(*part).map_or(true, |rig_ref| rig_ref.rig_root != binding.rig);
			let changed = binding.skins.iter().any(|bound| {
				skinned_meshes.get(bound.entity).map_or(true, |(skin, mesh)| {
					bound.mesh != mesh.map(|mesh| mesh.0.id())
						|| bound.inverse_bindposes != skin.inverse_bindposes.id()
				}) || bound.mesh.is_some_and(|mesh| changed_meshes.contains(&mesh))
					|| changed_binds.contains(&bound.inverse_bindposes)
			});
			(rebound || changed).then_some(*part)
		})
		.collect();
	for part in invalid {
		if let Some(binding) = tracker.parts.remove(&part) {
			if endpoint_components.get(binding.rig).is_ok_and(|endpoints| {
				endpoints
					.sources
					.iter()
					.flatten()
					.any(|source| binding.skins.iter().any(|skin| skin.entity == *source))
			}) {
				commands.entity(binding.rig).try_remove::<HumanoidForearmEndpoints>();
			}
			if all_parts.get(part).is_ok() {
				commands.entity(part).try_insert(NeedsSkinRemap);
			}
		}
	}
	for (part_root, children, part, rig_ref, _needs_remap) in &part_roots {
		let Ok(rig_map) = rig_maps.get(rig_ref.rig_root) else {
			continue;
		};
		if rig_map.by_name.is_empty() {
			continue;
		}

		let calibrate_body = part.slot == CharacterPartSlot::BodyMesh
			&& (rig_map.by_name.contains_key("forearm.L")
				|| rig_map.by_name.contains_key("forearm.R"));
		let mut endpoints = HumanoidForearmEndpoints::default();
		let mut geometry_pending = false;
		let mut geometry_binding = BodyEndpointBinding { rig: rig_ref.rig_root, skins: Vec::new() };
		if calibrate_body {
			commands.entity(rig_ref.rig_root).try_remove::<HumanoidForearmEndpoints>();
		}
		let mut stack: Vec<Entity> = children.iter().collect();
		let mut any_skinned = false;
		let mut all_meshes_ok = true;
		let mut missing_joints = HashSet::new();
		let mut remapped_meshes = Vec::new();

		while let Some(entity) = stack.pop() {
			if let Ok((mut skin, mesh_handle)) = skinned_meshes.get_mut(entity) {
				any_skinned = true;
				let mut new_joints = Vec::with_capacity(skin.joints.len());
				let mut mesh_ok = true;

				for old_joint in &skin.joints {
					let Ok(old_name) = names_q.get(*old_joint) else {
						mesh_ok = false;
						continue;
					};

					match rig_map.by_name.get(old_name.as_str()) {
						Some(new_joint) => new_joints.push(*new_joint),
						None => {
							missing_joints.insert(old_name.to_string());
							mesh_ok = false;
						}
					}
				}

				if mesh_ok && new_joints.len() == skin.joints.len() {
					if calibrate_body {
						geometry_binding.skins.push(BodySkinBinding {
							entity,
							mesh: mesh_handle.map(|mesh| mesh.0.id()),
							inverse_bindposes: skin.inverse_bindposes.id(),
						});
						if let Some(mesh_handle) = mesh_handle {
							if mesh_handle.0 == Handle::<Mesh>::default()
								|| skin.inverse_bindposes
									== Handle::<SkinnedMeshInverseBindposes>::default()
							{
								skin.joints = new_joints;
								remapped_meshes.push(entity);
								continue;
							}
							if let (Some(mesh_assets), Some(bind_assets)) =
								(mesh_assets.as_deref(), bind_assets.as_deref())
							{
								if let (Some(mesh), Some(bindposes)) = (
									mesh_assets.get(&mesh_handle.0),
									bind_assets.get(&skin.inverse_bindposes),
								) {
									let joints = ["forearm.L", "forearm.R"].map(|name| {
										rig_map.by_name.get(name).and_then(|target| {
											new_joints.iter().position(|joint| joint == target)
										})
									});
									for (side, point) in
										forearm_terminal_points(mesh, bindposes, joints)
											.into_iter()
											.enumerate()
									{
										if let Some(point) = point {
											if endpoints.points[side]
												.is_none_or(|previous| point.y > previous.y)
											{
												endpoints.points[side] = Some(point);
												endpoints.sources[side] = Some(entity);
											}
										}
									}
								} else {
									geometry_pending = true;
								}
							}
						}
					}
					skin.joints = new_joints;
					remapped_meshes.push(entity);
				} else {
					all_meshes_ok = false;
				}
			}

			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}

		if calibrate_body {
			tracker.parts.insert(part_root, geometry_binding);
			if !geometry_pending && all_meshes_ok {
				commands.entity(rig_ref.rig_root).try_insert(endpoints);
			}
		}
		if !any_skinned {
			let mut instance_ready = false;
			let mut found_instance = false;
			let mut instance_stack = vec![part_root];
			while let Some(current) = instance_stack.pop() {
				if let Ok(instance) = scene_instances.get(current) {
					found_instance = true;
					if scene_spawner.instance_is_ready(**instance) {
						instance_ready = true;
						break;
					}
				}
				if let Ok(children) = children_q.get(current) {
					instance_stack.extend(children.iter());
				}
			}
			if !found_instance || !instance_ready {
				continue;
			}
			commands.entity(part_root).try_remove::<NeedsSkinRemap>();
			continue;
		}

		if !all_meshes_ok {
			let mut missing: Vec<_> = missing_joints.into_iter().collect();
			missing.sort();
			warn!(
				"Character part {:?} skin remap failed ({} missing rig joints): {}",
				part.slot,
				missing.len(),
				missing.join(", ")
			);
			commands
				.entity(part_root)
				.try_insert(NoMatchingArmature { missing_joints: missing });
		} else {
			for entity in &remapped_meshes {
				commands.entity(*entity).try_insert(ChildOf(part_root));
			}
			commands
				.entity(part_root)
				.try_insert(NeedsDuplicateScenePrune { keep: remapped_meshes });
		}

		if !geometry_pending {
			commands.entity(part_root).try_remove::<NeedsSkinRemap>();
		}
	}
}

pub fn prune_duplicate_part_scenes(
	mut commands: Commands,
	part_roots: Query<(Entity, Option<&Children>, &NeedsDuplicateScenePrune), With<CharacterPart>>,
) {
	for (part_root, children, prune) in &part_roots {
		let keep: HashSet<_> = prune.keep.iter().copied().collect();
		if let Some(children) = children {
			for child in children.iter() {
				if !keep.contains(&child) {
					commands.entity(child).try_despawn();
				}
			}
		}
		commands.entity(part_root).try_remove::<NeedsDuplicateScenePrune>();
	}
}

#[cfg(test)]
mod tests;
