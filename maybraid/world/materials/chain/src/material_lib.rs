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
	use material_ref::{MaterialRef, MaterialRefApplied, MaterialRefPlugin, MaterialRefRoot};

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
}
