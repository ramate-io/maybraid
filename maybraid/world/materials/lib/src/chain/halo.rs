//! Additive halo shell. Gameplay cameras have no bloom; a Fresnel rim on the
//! opaque mesh cannot draw outside the metal silhouette. This child mesh uses
//! the same skinning path and a slightly expanded additive pass.
//!
//! Sync is event-driven. `apply_chain_hit` mutates the parent asset in place,
//! so `Changed<MeshMaterial3d>` is not enough. Halo-asset writes are ignored
//! so a settled chain stops modifying materials.

use std::collections::HashSet;

use bevy::asset::{AssetEvent, AssetId};
use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::light::NotShadowCaster;
use bevy::math::Vec3A;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;

use crate::chain::chain_material::{ChainMaterial, ChainMaterialUniform};
use crate::chain::flow::ChainFlowStyle;

/// Request a halo child on a chain mesh entity.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ChainHalo;

/// Child entity that draws the additive shell.
#[derive(Component, Clone, Debug)]
pub struct ChainHaloMesh {
	pub material: Handle<ChainMaterial>,
}

/// Halo material ids. Used to ignore feedback from our own writes.
#[derive(Resource, Default, Debug)]
pub struct HaloMaterialIds(pub HashSet<AssetId<ChainMaterial>>);

/// Default shell width in metres. Authored recipes override via
/// [`ChainFlowStyle::halo_width_m`].
pub const HALO_EXPANSION: f32 = ChainFlowStyle::DEFAULT_HALO_WIDTH_M;

type HaloParentQuery<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static Mesh3d,
		&'static MeshMaterial3d<ChainMaterial>,
		Option<&'static SkinnedMesh>,
		Option<&'static Children>,
	),
	With<ChainHalo>,
>;

fn uniforms_match_for_halo(src: &ChainMaterialUniform, dst: &ChainMaterialUniform) -> bool {
	src.colors == dst.colors
		&& src.noise == dst.noise
		&& src.scalars == dst.scalars
		&& src.rasters == dst.rasters
		&& src.impact == dst.impact
		&& src.link_damage == dst.link_damage
		&& dst.is_halo()
}

fn copy_runtime_onto_halo(dst: &mut ChainMaterial, src: &ChainMaterial) {
	dst.params.apply_authored(&src.params);
	dst.params.impact = src.params.impact;
	dst.params.link_damage = src.params.link_damage;
	dst.params.set_halo_pass(true);
	dst.params.set_halo_expansion(src.params.flow_halo_width());
}

fn expand_aabb(mesh: &Mesh, pad: f32) -> Option<Aabb> {
	let aabb = mesh.compute_aabb()?;
	Some(Aabb { center: aabb.center, half_extents: aabb.half_extents + Vec3A::splat(pad.max(0.0)) })
}

fn sync_skin(commands: &mut Commands, child: Entity, parent_skin: Option<&SkinnedMesh>) {
	match parent_skin {
		Some(skin) => {
			commands.entity(child).insert(SkinnedMesh {
				inverse_bindposes: skin.inverse_bindposes.clone(),
				joints: skin.joints.clone(),
			});
		}
		None => {
			commands.entity(child).remove::<SkinnedMesh>();
		}
	}
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_chain_halos(
	mut commands: Commands,
	mut materials: ResMut<Assets<ChainMaterial>>,
	mut halo_ids: ResMut<HaloMaterialIds>,
	meshes: Res<Assets<Mesh>>,
	mut events: MessageReader<AssetEvent<ChainMaterial>>,
	parents: HaloParentQuery,
	halos: Query<&ChainHaloMesh>,
	changed_mesh: Query<Entity, (With<ChainHalo>, Changed<Mesh3d>)>,
	changed_mat: Query<Entity, (With<ChainHalo>, Changed<MeshMaterial3d<ChainMaterial>>)>,
	changed_skin: Query<Entity, (With<ChainHalo>, Changed<SkinnedMesh>)>,
	added: Query<Entity, Added<ChainHalo>>,
) {
	let mut dirty_parents: HashSet<AssetId<ChainMaterial>> = HashSet::new();
	for event in events.read() {
		if let AssetEvent::Modified { id } = event {
			if !halo_ids.0.contains(id) {
				dirty_parents.insert(*id);
			}
		}
	}

	for (entity, mesh, parent_mat, parent_skin, children) in &parents {
		let Some(src) = materials.get(&parent_mat.0).cloned() else {
			continue;
		};
		let existing = children.and_then(|kids| {
			kids.iter()
				.find_map(|child| halos.get(child).ok().map(|halo| (child, halo.material.clone())))
		});
		let force_spawn = added.get(entity).is_ok();
		let mesh_changed = changed_mesh.get(entity).is_ok();
		let mat_changed = changed_mat.get(entity).is_ok();
		let skin_changed = changed_skin.get(entity).is_ok();
		let parent_dirty = dirty_parents.contains(&parent_mat.0.id());

		let halo_handle = if let Some((_, handle)) = existing.clone() {
			handle
		} else {
			let mut halo = src.clone();
			halo.params.set_halo_pass(true);
			halo.params.set_halo_expansion(src.params.flow_halo_width());
			let handle = materials.add(halo);
			halo_ids.0.insert(handle.id());
			handle
		};

		if existing.is_none() || force_spawn && existing.is_none() {
			let pad = src.params.flow_halo_width();
			let aabb = meshes.get(&mesh.0).and_then(|m| expand_aabb(m, pad));
			let mut child = commands.spawn((
				Name::new("chain-halo"),
				ChainHaloMesh { material: halo_handle.clone() },
				Mesh3d(mesh.0.clone()),
				MeshMaterial3d(halo_handle.clone()),
				NotShadowCaster,
				Transform::IDENTITY,
				Visibility::default(),
			));
			if let Some(aabb) = aabb {
				child.insert(aabb);
			}
			let child_id = child.id();
			if let Some(skin) = parent_skin {
				commands.entity(child_id).insert(SkinnedMesh {
					inverse_bindposes: skin.inverse_bindposes.clone(),
					joints: skin.joints.clone(),
				});
			}
			commands.entity(entity).add_child(child_id);
			continue;
		}

		let Some((child, _)) = existing else {
			continue;
		};
		if mesh_changed {
			let pad = src.params.flow_halo_width();
			let mut entity_cmds = commands.entity(child);
			entity_cmds.insert(Mesh3d(mesh.0.clone()));
			if let Some(aabb) = meshes.get(&mesh.0).and_then(|m| expand_aabb(m, pad)) {
				entity_cmds.insert(aabb);
			}
		}
		if skin_changed || parent_skin.is_some() && added.get(entity).is_ok() {
			sync_skin(&mut commands, child, parent_skin);
		}
		if mat_changed || parent_dirty {
			if let Some(halo) = materials.get(&halo_handle) {
				if uniforms_match_for_halo(&src.params, &halo.params) {
					continue;
				}
			}
			if let Some(mut halo) = materials.get_mut(&halo_handle) {
				copy_runtime_onto_halo(&mut halo, &src);
			}
		}
	}
}

pub(crate) fn cleanup_removed_halos(
	mut commands: Commands,
	mut halo_ids: ResMut<HaloMaterialIds>,
	orphans: Query<(Entity, &ChainHaloMesh, &ChildOf)>,
	parents: Query<(), With<ChainHalo>>,
) {
	for (entity, halo, child_of) in &orphans {
		if parents.get(child_of.parent()).is_ok() {
			continue;
		}
		halo_ids.0.remove(&halo.material.id());
		commands.entity(entity).try_despawn();
	}
}

/// Gentle expansion used by tests. Authored width plus a small energy term.
pub fn halo_expansion_metres(energy: f32, base: f32) -> f32 {
	base.max(0.0) * (0.94 + 0.08 * energy.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::chain::flow::{chain_flow, ChainFlowStyle};

	#[test]
	fn halo_expansion_stays_near_the_authored_width() -> anyhow::Result<()> {
		let flow = ChainFlowStyle::default();
		let trough = halo_expansion_metres(flow.baseline, flow.halo_width_m);
		let crest = halo_expansion_metres(flow.peak, flow.halo_width_m);
		assert!(crest >= trough);
		assert!(crest < flow.halo_width_m * 1.12);
		assert!(trough > flow.halo_width_m * 0.90);
		let moving = chain_flow(0.4, 0.2, flow);
		assert!((halo_expansion_metres(moving, flow.halo_width_m) - trough).abs() < 0.004);
		Ok(())
	}

	#[test]
	fn settled_chains_stop_modifying_halo_assets() -> anyhow::Result<()> {
		use crate::chain::params::mythical_chain_ref;
		use crate::chain::placeholder::{placeholder_chain_mesh, PlaceholderChainSpec};
		use crate::chain::runtime::{
			sync_owned_chain_materials, OwnedChainHandle, OwnedChainMaterial,
		};
		use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
		use material_ref::{MaterialRefPlugin, MaterialRefRoot};

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<ChainMaterial>()
			.init_asset::<SkinnedMeshInverseBindposes>()
			.init_resource::<crate::chain::ChainMaterialRefCache>()
			.init_resource::<HaloMaterialIds>()
			.add_plugins(MaterialRefPlugin::<crate::chain::ChainMaterialLib<'_>>::default())
			.add_systems(Update, (sync_owned_chain_materials, sync_chain_halos).chain());

		let mesh = app
			.world_mut()
			.resource_mut::<Assets<Mesh>>()
			.add(placeholder_chain_mesh(PlaceholderChainSpec::default().with_link_count(4)));
		let parent = app
			.world_mut()
			.spawn((
				MaterialRefRoot(mythical_chain_ref()),
				Mesh3d(mesh),
				OwnedChainMaterial,
				ChainHalo,
			))
			.id();
		for _ in 0..4 {
			app.update();
		}
		let owned = app
			.world()
			.entity(parent)
			.get::<OwnedChainHandle>()
			.ok_or_else(|| anyhow::anyhow!("owned"))?
			.0
			.clone();
		{
			let mut materials = app.world_mut().resource_mut::<Assets<ChainMaterial>>();
			let Some(mut material) = materials.get_mut(&owned) else {
				return Err(anyhow::anyhow!("owned material"));
			};
			crate::chain::runtime::apply_chain_hit(&mut material, 1, 0.4, 4.0, 0.5);
		}
		app.update();
		app.update();
		let snapshot = {
			let materials = app.world().resource::<Assets<ChainMaterial>>();
			let mut found = None;
			for (_id, mat) in materials.iter() {
				if mat.params.is_halo() {
					found = Some(mat.params);
					break;
				}
			}
			found.ok_or_else(|| anyhow::anyhow!("halo material"))?
		};
		for _ in 0..6 {
			app.update();
		}
		let after = {
			let materials = app.world().resource::<Assets<ChainMaterial>>();
			let mut found = None;
			for (_id, mat) in materials.iter() {
				if mat.params.is_halo() {
					found = Some(mat.params);
					break;
				}
			}
			found.ok_or_else(|| anyhow::anyhow!("halo after"))?
		};
		assert_eq!(snapshot, after, "settled chain must not keep rewriting the halo");
		assert_eq!(app.world().resource::<HaloMaterialIds>().0.len(), 1);
		Ok(())
	}

	#[test]
	fn halo_copies_parent_skin_and_cleans_up() -> anyhow::Result<()> {
		use crate::chain::params::mythical_chain_ref;
		use crate::chain::string::{skinned_tube_mesh, ContinuousTubeSpec};
		use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
		use material_ref::{MaterialRefPlugin, MaterialRefRoot};

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<ChainMaterial>()
			.init_asset::<SkinnedMeshInverseBindposes>()
			.init_resource::<crate::chain::ChainMaterialRefCache>()
			.init_resource::<HaloMaterialIds>()
			.add_plugins(MaterialRefPlugin::<crate::chain::ChainMaterialLib<'_>>::default())
			.add_systems(Update, (sync_chain_halos, cleanup_removed_halos).chain());

		let (mesh, inverses) = skinned_tube_mesh(ContinuousTubeSpec::default());
		let mesh_h = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
		let bind = app
			.world_mut()
			.resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
			.add(SkinnedMeshInverseBindposes::from(inverses));
		let j0 = app.world_mut().spawn(Transform::IDENTITY).id();
		let j1 = app.world_mut().spawn(Transform::IDENTITY).id();
		let j2 = app.world_mut().spawn(Transform::IDENTITY).id();
		let j3 = app.world_mut().spawn(Transform::IDENTITY).id();
		let parent = app
			.world_mut()
			.spawn((
				MaterialRefRoot(mythical_chain_ref()),
				Mesh3d(mesh_h),
				SkinnedMesh { inverse_bindposes: bind, joints: vec![j0, j1, j2, j3] },
				ChainHalo,
			))
			.id();
		app.update();
		app.update();
		let child = app
			.world()
			.entity(parent)
			.get::<Children>()
			.and_then(|kids| kids.iter().next())
			.ok_or_else(|| anyhow::anyhow!("halo child"))?;
		let joints = app
			.world()
			.entity(child)
			.get::<SkinnedMesh>()
			.map(|skin| skin.joints.len())
			.ok_or_else(|| anyhow::anyhow!("child skin"))?;
		assert_eq!(joints, 4, "halo must copy parent joints");
		app.world_mut().entity_mut(parent).remove::<ChainHalo>();
		app.update();
		app.update();
		assert!(app.world().get_entity(child).is_err(), "halo child should despawn");
		Ok(())
	}
}

pub(crate) fn register_halo_systems(app: &mut App) {
	app.init_resource::<HaloMaterialIds>().add_systems(
		Update,
		(
			sync_chain_halos.after(crate::chain::runtime::sync_owned_chain_materials),
			cleanup_removed_halos.after(sync_chain_halos),
		),
	);
}
