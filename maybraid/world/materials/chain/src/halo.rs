//! Additive halo shell. Gameplay cameras have no bloom; a Fresnel rim on the
//! opaque mesh cannot draw outside the metal silhouette. This child mesh uses
//! the same skinning path and a slightly expanded additive pass.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::chain_material::ChainMaterial;
use crate::cracks::ChainCrackAtlas;
use crate::pulse::chain_pulse;

/// Request a halo child on a chain mesh entity.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ChainHalo;

/// Child entity that draws the additive shell.
#[derive(Component, Clone, Debug)]
pub struct ChainHaloMesh {
	pub material: Handle<ChainMaterial>,
}

/// Default shell expansion in metres at pulse crest.
pub const HALO_EXPANSION: f32 = 0.034;

type HaloParentQuery<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static Mesh3d, &'static MeshMaterial3d<ChainMaterial>, Option<&'static Children>),
	With<ChainHalo>,
>;

pub(crate) fn sync_chain_halos(
	mut commands: Commands,
	mut materials: ResMut<Assets<ChainMaterial>>,
	mut atlas: ResMut<ChainCrackAtlas>,
	mut images: ResMut<Assets<Image>>,
	parents: HaloParentQuery,
	halos: Query<&ChainHaloMesh>,
) {
	let cracks = atlas.get_or_create(&mut images);
	for (entity, mesh, parent_mat, children) in &parents {
		let Some(src) = materials.get(&parent_mat.0).cloned() else {
			continue;
		};
		let existing = children.and_then(|kids| {
			kids.iter()
				.find_map(|child| halos.get(child).ok().map(|halo| (child, halo.material.clone())))
		});
		let halo_handle = if let Some((_, handle)) = existing.clone() {
			handle
		} else {
			let mut halo = src.clone();
			halo.cracks =
				if src.cracks == Handle::default() { cracks.clone() } else { src.cracks.clone() };
			halo.params.set_halo_pass(true);
			halo.params.set_halo_expansion(HALO_EXPANSION);
			materials.add(halo)
		};
		if let Some(mut halo) = materials.get_mut(&halo_handle) {
			halo.params.apply_authored(&src.params);
			halo.params.impact = src.params.impact;
			halo.params.link_damage = src.params.link_damage;
			halo.params.set_halo_pass(true);
			halo.params.set_halo_expansion(HALO_EXPANSION);
			if halo.cracks == Handle::default() {
				halo.cracks = cracks.clone();
			}
		}
		if existing.is_none() {
			let child = commands
				.spawn((
					Name::new("chain-halo"),
					ChainHaloMesh { material: halo_handle.clone() },
					Mesh3d(mesh.0.clone()),
					MeshMaterial3d(halo_handle),
					NotShadowCaster,
					Transform::IDENTITY,
					Visibility::default(),
				))
				.id();
			commands.entity(entity).add_child(child);
		}
	}
}

/// Pulse-driven expansion used by tests and the vertex stage.
pub fn halo_expansion_metres(time: f32, pulse_rate: f32, base: f32) -> f32 {
	let pulse = chain_pulse(time, pulse_rate, 0.0);
	base * (0.55 + 0.80 * pulse)
}

pub(crate) fn register_halo_systems(app: &mut App) {
	app.add_systems(Update, sync_chain_halos.after(crate::runtime::sync_owned_chain_materials));
}
