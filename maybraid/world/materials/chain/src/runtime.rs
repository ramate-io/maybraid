//! Owned runtime [`ChainMaterial`] handles and per-link damage uniforms.
//!
//! Authored look (palette, seed, intact / fade / glow) stays on
//! [`material_ref::MaterialRef`]. Independent damage lives on a unique
//! [`Handle<ChainMaterial>`] updated in place so restamps do not fill the
//! shared [`crate::ChainMaterialRefCache`].

use bevy::prelude::*;
use material_ref::MaterialRefSystems;

use crate::chain_material::{ChainMaterial, ChainMaterialUniform, MAX_CHAIN_LINKS};
use crate::cracks::ChainCrackAtlas;

/// Mark a chain that must own a unique, mutable [`ChainMaterial`].
///
/// Shared decorative instances omit this and keep using the
/// [`material_ref::MaterialRefKey`] cache.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct OwnedChainMaterial;

/// Handle stored after the first owned upgrade. Survives MaterialRef restamps.
#[derive(Component, Clone, Debug)]
pub struct OwnedChainHandle(pub Handle<ChainMaterial>);

/// Packed persistent damage for [`MAX_CHAIN_LINKS`] links plus one hit flash.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainRuntimeDamage {
	pub link_damage: [f32; MAX_CHAIN_LINKS],
	/// Along-chain hit position in `[0, 1]`.
	pub impact_pos: f32,
	pub impact_radius: f32,
	pub impact_strength: f32,
	pub impact_time: f32,
}

impl Default for ChainRuntimeDamage {
	fn default() -> Self {
		Self {
			link_damage: [0.0; MAX_CHAIN_LINKS],
			impact_pos: 0.0,
			impact_radius: 0.08,
			impact_strength: 0.0,
			impact_time: 0.0,
		}
	}
}

impl ChainRuntimeDamage {
	pub fn link(&self, link_id: u32) -> f32 {
		self.link_damage.get(link_id as usize).copied().unwrap_or(0.0)
	}

	pub fn set_link(&mut self, link_id: u32, value: f32) {
		if let Some(slot) = self.link_damage.get_mut(link_id as usize) {
			*slot = value.clamp(0.0, 1.0);
		}
	}

	pub fn add_link(&mut self, link_id: u32, delta: f32) {
		self.set_link(link_id, self.link(link_id) + delta);
	}

	/// Persistent damage on `link_id` plus a decaying hotspot.
	pub fn hit(&mut self, link_id: u32, amount: f32, link_count: f32, time: f32) {
		self.add_link(link_id, amount);
		let n = link_count.max(1.0);
		self.impact_pos = ((link_id as f32 + 0.5) / n).clamp(0.0, 1.0);
		self.impact_radius = 0.085;
		self.impact_strength = amount.clamp(0.0, 1.0).max(0.55);
		self.impact_time = time;
	}

	pub fn write_to(&self, params: &mut ChainMaterialUniform) {
		params.set_link_damage_array(&self.link_damage);
		params.set_impact(
			self.impact_pos,
			self.impact_radius,
			self.impact_strength,
			self.impact_time,
		);
	}

	pub fn from_uniform(params: &ChainMaterialUniform) -> Self {
		let mut link_damage = [0.0; MAX_CHAIN_LINKS];
		params.copy_link_damage_array(&mut link_damage);
		Self {
			link_damage,
			impact_pos: params.impact.x,
			impact_radius: params.impact.y,
			impact_strength: params.impact.z,
			impact_time: params.impact.w,
		}
	}
}

/// Apply a hit to an owned material in place. No cache insert.
pub fn apply_chain_hit(
	material: &mut ChainMaterial,
	link_id: u32,
	amount: f32,
	link_count: f32,
	time: f32,
) {
	let mut runtime = ChainRuntimeDamage::from_uniform(&material.params);
	runtime.hit(link_id, amount, link_count, time);
	runtime.write_to(&mut material.params);
}

/// Allocate a unique material cloned from `src`. Not inserted into the cache.
pub fn owned_chain_material(
	materials: &mut Assets<ChainMaterial>,
	src: &ChainMaterial,
) -> Handle<ChainMaterial> {
	materials.add(src.clone())
}

/// Clone authored fields from `src` onto `dst`, keeping runtime damage / halo pass.
pub fn copy_authored_onto(dst: &mut ChainMaterial, src: &ChainMaterial) {
	dst.params.apply_authored(&src.params);
	if dst.cracks == Handle::default() {
		dst.cracks = src.cracks.clone();
	}
}

type OwnedChainQuery<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static MeshMaterial3d<ChainMaterial>, Option<&'static OwnedChainHandle>),
	With<OwnedChainMaterial>,
>;

pub(crate) fn sync_owned_chain_materials(
	mut commands: Commands,
	mut materials: ResMut<Assets<ChainMaterial>>,
	mut atlas: ResMut<ChainCrackAtlas>,
	mut images: ResMut<Assets<Image>>,
	query: OwnedChainQuery,
) {
	let cracks = atlas.get_or_create(&mut images);
	for (entity, current, owned) in &query {
		if let Some(owned) = owned {
			if current.0 != owned.0 {
				if let (Some(src), Some(mut dst)) =
					(materials.get(&current.0).cloned(), materials.get_mut(&owned.0))
				{
					copy_authored_onto(&mut dst, &src);
					if dst.cracks == Handle::default() {
						dst.cracks = cracks.clone();
					}
				}
				commands.entity(entity).insert(MeshMaterial3d(owned.0.clone()));
			}
			continue;
		}
		let Some(shared) = materials.get(&current.0).cloned() else {
			continue;
		};
		let mut clone = shared;
		if clone.cracks == Handle::default() {
			clone.cracks = cracks.clone();
		}
		let unique = materials.add(clone);
		commands
			.entity(entity)
			.insert(MeshMaterial3d(unique.clone()))
			.insert(OwnedChainHandle(unique));
	}
}

pub(crate) fn ensure_crack_atlas_on_materials(
	mut materials: ResMut<Assets<ChainMaterial>>,
	mut atlas: ResMut<ChainCrackAtlas>,
	mut images: ResMut<Assets<Image>>,
) {
	let cracks = atlas.get_or_create(&mut images);
	for (_, material) in materials.iter_mut() {
		if material.cracks == Handle::default() {
			material.cracks = cracks.clone();
		}
	}
}

/// Install owned-handle sync after MaterialRef fulfill.
pub(crate) fn register_runtime_systems(app: &mut App) {
	app.add_systems(
		Update,
		(
			ensure_crack_atlas_on_materials,
			sync_owned_chain_materials
				.after(MaterialRefSystems::Fulfill)
				.after(ensure_crack_atlas_on_materials),
		),
	);
}
