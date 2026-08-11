//! Material loading via references — named recipe + palette + noise → shared handle.
//!
//! Parallel to [`scene_ref`](scene_ref) and LOD’s [`LodSceneRegionIndex`](lod::LodSceneRegionIndex):
//!
//! - [`MaterialRef`] / [`MaterialRefRoot`] — deferred identity
//! - [`MaterialLib`] — capability trait implemented by a `#[derive(SystemParam)]` item
//! - [`MaterialRefPlugin`]`<L>` — fulfill system generic over that SystemParam
//! - [`StandardMaterialLib`] / [`StandardMaterialRefPlugin`] — reference implementor
//!
//! Domain crates add their own SystemParam libs (e.g. Chico leaf + stick) that fork on
//! [`MaterialId`] and insert the matching `MeshMaterial3d<M>`.

mod fulfill;
mod key;
mod lib_trait;
mod material_ref;
mod reference;
mod standard;

pub use fulfill::{
	fulfill_material_ref_descendants, fulfill_material_ref_roots, MaterialRefPlugin,
};
pub use key::{hash_material_ref, MaterialRefCache, MaterialRefKey, NoiseParamsKey};
pub use lib_trait::MaterialLib;
pub use material_ref::{
	MaterialId, MaterialRef, MaterialRefApplied, MaterialRefRoot, PropagateToDescendants,
};
pub use reference::ReferenceMaterial;
pub use standard::{StandardMaterialLib, StandardMaterialRefCache, StandardMaterialRefPlugin};

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::*;
	use procedural_common::NoiseParams;

	#[test]
	fn material_ref_key_stable_for_same_inputs() -> anyhow::Result<()> {
		let a = MaterialRef::named("tuft")
			.with_palette([Color::srgb(0.1, 0.5, 0.2)])
			.with_noise(NoiseParams::from_scalar(3.0, 1.0, 0.2, 1));
		let b = MaterialRef::named("tuft")
			.with_palette([Color::srgb(0.1, 0.5, 0.2)])
			.with_noise(NoiseParams::from_scalar(3.0, 1.0, 0.2, 1));
		assert_eq!(MaterialRefKey::from(&a), MaterialRefKey::from(&b));
		assert_eq!(hash_material_ref(&a), hash_material_ref(&b));
		Ok(())
	}

	#[test]
	fn material_ref_key_changes_with_palette_or_name() -> anyhow::Result<()> {
		let base = MaterialRef::named("tuft").with_palette([Color::srgb(0.1, 0.5, 0.2)]);
		let other_color =
			MaterialRef::named("tuft").with_palette([Color::srgb(0.9, 0.1, 0.1)]);
		let other_name =
			MaterialRef::named("bark").with_palette([Color::srgb(0.1, 0.5, 0.2)]);
		assert_ne!(MaterialRefKey::from(&base), MaterialRefKey::from(&other_color));
		assert_ne!(MaterialRefKey::from(&base), MaterialRefKey::from(&other_name));
		Ok(())
	}

	#[test]
	fn standard_lib_fulfills_and_caches() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_resource::<StandardMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<StandardMaterialLib<'_>>::default());

		let entity = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named("tuft"))).id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).is_some());
		assert_eq!(app.world().resource::<StandardMaterialRefCache>().len(), 1);

		// Second entity with same ref reuses cache.
		let entity2 = app
			.world_mut()
			.spawn(MaterialRefRoot(MaterialRef::named("tuft")))
			.id();
		app.update();
		assert!(app.world().get::<MaterialRefApplied>(entity2).is_some());
		assert_eq!(app.world().resource::<StandardMaterialRefCache>().len(), 1);
		Ok(())
	}

	#[test]
	fn propagate_fulfills_mesh_descendants() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_resource::<StandardMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<StandardMaterialLib<'_>>::default());

		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Mesh::from(
			bevy::prelude::Cuboid::from_length(1.0),
		));
		let root = app
			.world_mut()
			.spawn((
				MaterialRefRoot(MaterialRef::named("tuft")),
				PropagateToDescendants,
			))
			.id();
		app.update();
		assert!(app.world().get::<MaterialRefApplied>(root).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(root).is_none());

		let child = app
			.world_mut()
			.spawn((Mesh3d(mesh), ChildOf(root)))
			.id();
		app.update();
		assert!(app.world().get::<MaterialRefApplied>(child).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(child).is_some());
		Ok(())
	}
}
