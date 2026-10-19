//! [`MaterialLib`] that claims only `mythical_chain`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
};

use crate::chain_material::{claims_recipe, ChainMaterial};

/// Cache of resolved [`ChainMaterial`] handles.
pub type ChainMaterialRefCache = MaterialRefCache<ChainMaterial>;

/// Inserts the chain-material cache. Idempotent.
pub fn init_chain_material_caches(app: &mut App) {
	app.init_resource::<ChainMaterialRefCache>();
}

pub(crate) fn resolve_chain(
	materials: &mut Assets<ChainMaterial>,
	cache: &mut ChainMaterialRefCache,
	material_ref: &MaterialRef,
) -> Handle<ChainMaterial> {
	let key = MaterialRefKey::from(material_ref);
	if let Some(handle) = cache.get(&key) {
		return handle;
	}
	let handle = materials.add(ChainMaterial::from_material_ref(material_ref));
	cache.insert(key, handle.clone());
	handle
}

/// Claims `mythical_chain` only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct ChainMaterialLib<'w> {
	pub materials: ResMut<'w, Assets<ChainMaterial>>,
	pub cache: ResMut<'w, ChainMaterialRefCache>,
}

impl MaterialLib for ChainMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if claims_recipe(name) => {
				let handle = resolve_chain(&mut self.materials, &mut self.cache, material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Chain recipe, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct ChainStandaloneMaterialLib<'w> {
	pub chain: ChainMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for ChainStandaloneMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.chain.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers caches + [`MaterialRefPlugin`] for [`ChainStandaloneMaterialLib`].
///
/// Composed apps that already installed a fulfill plugin (Maybraid World) skip
/// here; they nest [`ChainMaterialLib`] instead.
pub struct ChainMaterialRefPlugin;

impl Plugin for ChainMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<StandardMaterialRefCache>();
		init_chain_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<ChainStandaloneMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use material_ref::{MaterialRefApplied, MaterialRefPlugin, MaterialRefRoot};

	use super::*;
	use crate::params::{
		mythical_chain_ref, mythical_chain_ref_for, MythicalChainState, RECIPE_MYTHICAL_CHAIN,
	};

	#[test]
	fn chain_lib_fulfills_mythical_chain() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<ChainMaterial>()
			.init_resource::<ChainMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<ChainMaterialLib<'_>>::default());

		let entity = app.world_mut().spawn(MaterialRefRoot(mythical_chain_ref())).id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<ChainMaterial>>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).is_none());
		Ok(())
	}

	#[test]
	fn two_durabilities_cache_two_handles() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<ChainMaterial>()
			.init_resource::<ChainMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<ChainMaterialLib<'_>>::default());

		app.world_mut()
			.spawn(MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Intact)));
		app.world_mut()
			.spawn(MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Damaged)));
		app.update();
		assert_eq!(app.world().resource::<ChainMaterialRefCache>().len(), 2);

		app.world_mut()
			.spawn(MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Intact)));
		app.update();
		assert_eq!(app.world().resource::<ChainMaterialRefCache>().len(), 2);
		assert_eq!(RECIPE_MYTHICAL_CHAIN, "mythical_chain");
		Ok(())
	}

	#[test]
	fn owned_damage_does_not_accumulate_or_mutate_cached_handles() -> anyhow::Result<()> {
		use crate::runtime::{
			apply_chain_hit, sync_owned_chain_materials, OwnedChainHandle, OwnedChainMaterial,
		};

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<ChainMaterial>()
			.init_resource::<ChainMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<ChainMaterialLib<'_>>::default())
			.add_systems(Update, sync_owned_chain_materials);

		let shared_a = app
			.world_mut()
			.spawn(MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Intact)))
			.id();
		let shared_b = app
			.world_mut()
			.spawn(MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Intact)))
			.id();
		let owned = app
			.world_mut()
			.spawn((
				MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Intact)),
				OwnedChainMaterial,
			))
			.id();
		app.update();
		app.update();

		assert_eq!(app.world().resource::<ChainMaterialRefCache>().len(), 1);
		let shared_handle = app
			.world()
			.get::<MeshMaterial3d<ChainMaterial>>(shared_a)
			.ok_or_else(|| anyhow::anyhow!("shared a"))?
			.0
			.clone();
		let shared_b_handle = app
			.world()
			.get::<MeshMaterial3d<ChainMaterial>>(shared_b)
			.ok_or_else(|| anyhow::anyhow!("shared b"))?
			.0
			.clone();
		let owned_handle = app
			.world()
			.get::<OwnedChainHandle>(owned)
			.ok_or_else(|| anyhow::anyhow!("owned handle"))?
			.0
			.clone();
		assert_eq!(shared_handle, shared_b_handle);
		assert_ne!(shared_handle, owned_handle);

		let cache_len = app.world().resource::<ChainMaterialRefCache>().len();
		{
			let mut materials = app.world_mut().resource_mut::<Assets<ChainMaterial>>();
			let Some(mut material) = materials.get_mut(&owned_handle) else {
				return Err(anyhow::anyhow!("owned material"));
			};
			apply_chain_hit(&mut material, 4, 0.35, 14.0, 1.0);
			apply_chain_hit(&mut material, 4, 0.25, 14.0, 2.0);
			apply_chain_hit(&mut material, 7, 0.5, 14.0, 3.0);
		}
		app.world_mut()
			.entity_mut(owned)
			.insert(MaterialRefRoot(mythical_chain_ref_for(MythicalChainState::Damaged)));
		app.update();
		app.update();

		assert_eq!(app.world().resource::<ChainMaterialRefCache>().len(), cache_len + 1);
		let owned_after = app
			.world()
			.get::<OwnedChainHandle>(owned)
			.ok_or_else(|| anyhow::anyhow!("owned after"))?
			.0
			.clone();
		assert_eq!(owned_after, owned_handle);
		let materials = app.world().resource::<Assets<ChainMaterial>>();
		let owned_mat = materials.get(&owned_handle).ok_or_else(|| anyhow::anyhow!("owned mat"))?;
		let shared_mat =
			materials.get(&shared_handle).ok_or_else(|| anyhow::anyhow!("shared mat"))?;
		assert!((owned_mat.params.link_damage(4) - 0.6).abs() < 1e-5);
		assert!((owned_mat.params.link_damage(7) - 0.5).abs() < 1e-5);
		assert_eq!(shared_mat.params.link_damage(4), 0.0);
		assert!((owned_mat.params.intact() - MythicalChainState::Damaged.intact()).abs() < 1e-5);
		let look = crate::deteriorate::LinkLook::from_severity(
			crate::deteriorate::severity(
				owned_mat.params.damage().max(1.0 - owned_mat.params.intact()),
				0.6,
				0.4,
			),
			owned_mat.params.fade(),
			4.0,
			11.0,
		);
		assert!(look.removed_chunks >= 2, "restamped local damage must keep chunks missing");
		Ok(())
	}
}
