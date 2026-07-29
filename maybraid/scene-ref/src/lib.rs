//! Scene loading via references to avoid loading the same scene multiple times.
//!
//! [`SceneRef`] names a GLB (optionally with a `#SceneN` label) and an optional
//! [`MirrorAxis`]. [`SceneRefPlugin`] installs [`SceneRefHandles`], which memoizes
//! [`Handle<WorldAsset>`]s so repeated scene spawns share one load. Mirrored refs
//! rebuild meshes (axis flip + winding reverse) into a distinct cached asset.
//! Use [`SceneRef::scene`] / [`SceneRefRoot`]; the fulfill system inserts
//! [`WorldAssetRoot`] when the handle is ready.

use std::path::Path;

use bevy::asset::{AssetId, AssetPath, AssetServer, Handle, HandleTemplate};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::{
	App, Assets, Commands, Component, Entity, Mesh3d, Plugin, Query, Res, ResMut, Resource,
	Transform, Update, Vec3, Without,
};
use bevy::scene::prelude::{bsn, template_value};
use bevy::scene::Scene;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

/// Axis along which a [`SceneRef`] rebuilds mirrored mesh geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MirrorAxis {
	X,
	Y,
	Z,
}

impl MirrorAxis {
	fn scale(self) -> Vec3 {
		match self {
			Self::X => Vec3::new(-1.0, 1.0, 1.0),
			Self::Y => Vec3::new(1.0, -1.0, 1.0),
			Self::Z => Vec3::new(1.0, 1.0, -1.0),
		}
	}
}

/// Reference to a scene asset that can be resolved to a shared handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SceneRef {
	/// Path to a `.glb` (or other glTF) asset. If the path has no `#` label,
	/// scene `0` is used (`path#Scene0`).
	pub path: String,
	/// When set, resolve to a rebuilt [`WorldAsset`] mirrored on this axis.
	pub mirror: Option<MirrorAxis>,
}

/// BSN / ECS root that resolves to [`WorldAssetRoot`] via [`SceneRefHandles`].
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SceneRefRoot(pub SceneRef);

impl SceneRef {
	/// GLB / glTF at `path` (scene 0 unless `path` already includes a label).
	pub fn glb(path: impl Into<String>) -> Self {
		Self {
			path: path.into(),
			mirror: None,
		}
	}

	/// Same path with axis mirroring enabled.
	pub fn mirrored(mut self, axis: MirrorAxis) -> Self {
		self.mirror = Some(axis);
		self
	}

	/// Set or clear the mirror axis.
	pub fn with_mirror(mut self, mirror: Option<MirrorAxis>) -> Self {
		self.mirror = mirror;
		self
	}

	/// Source (unmirrored) ref sharing this path.
	pub fn without_mirror(&self) -> Self {
		Self {
			path: self.path.clone(),
			mirror: None,
		}
	}

	/// Asset path string used for loading the **source** glTF (includes `#Scene0` when unlabeled).
	pub fn labeled_path(&self) -> String {
		if self.path.contains('#') {
			self.path.clone()
		} else {
			format!("{}#Scene0", self.path)
		}
	}

	/// Load (or reuse via [`AssetServer`]) the source [`WorldAsset`] for this path.
	pub fn load_source(&self, asset_server: &AssetServer) -> Handle<WorldAsset> {
		asset_server.load(self.labeled_path())
	}

	/// BSN scene root for this ref (`SceneRefRoot`; fulfilled to [`WorldAssetRoot`]).
	pub fn scene(self) -> impl Scene + 'static {
		bsn! {
			template_value(SceneRefRoot(self))
		}
	}
}

impl From<&str> for SceneRef {
	fn from(path: &str) -> Self {
		Self::glb(path)
	}
}

impl From<String> for SceneRef {
	fn from(path: String) -> Self {
		Self::glb(path)
	}
}

impl From<&Path> for SceneRef {
	fn from(path: &Path) -> Self {
		Self::glb(path.to_string_lossy())
	}
}

impl From<SceneRef> for HandleTemplate<WorldAsset> {
	fn from(scene_ref: SceneRef) -> Self {
		assert!(
			scene_ref.mirror.is_none(),
			"mirrored SceneRef cannot convert to HandleTemplate::Path; use SceneRef::scene()"
		);
		HandleTemplate::Path(AssetPath::from(scene_ref.labeled_path()))
	}
}

impl From<&SceneRef> for HandleTemplate<WorldAsset> {
	fn from(scene_ref: &SceneRef) -> Self {
		assert!(
			scene_ref.mirror.is_none(),
			"mirrored SceneRef cannot convert to HandleTemplate::Path; use SceneRef::scene()"
		);
		HandleTemplate::Path(AssetPath::from(scene_ref.labeled_path()))
	}
}

/// Clone `mesh`, negate `axis` on positions/normals/tangents, and reverse winding.
pub fn mirror_mesh(mesh: &Mesh, axis: MirrorAxis) -> Mesh {
	let mut out = mesh.clone();
	out.transform_by(Transform::from_scale(axis.scale()));
	// Odd negative scale reverses winding; restore front-face orientation.
	let _ = out.invert_winding();
	out
}

/// Clone `source` and rewrite every `Mesh3d` to a newly registered mirrored mesh.
///
/// Caller must ensure the source handle is
/// [`AssetServer::is_loaded_with_dependencies`] so mesh bytes are in `Assets<Mesh>`.
fn mirror_world_asset(
	source: &WorldAsset,
	axis: MirrorAxis,
	meshes: &mut Assets<Mesh>,
	type_registry: &AppTypeRegistry,
) -> Option<WorldAsset> {
	let mut cloned = source.clone_with(type_registry).ok()?;

	let mut entities = Vec::new();
	for entity in cloned.world.iter_entities() {
		if let Some(mesh3d) = entity.get::<Mesh3d>() {
			entities.push((entity.id(), mesh3d.0.clone()));
		}
	}

	let mut remap: HashMap<AssetId<Mesh>, Handle<Mesh>> = HashMap::default();
	for (entity, old_handle) in entities {
		let new_handle = if let Some(h) = remap.get(&old_handle.id()) {
			h.clone()
		} else {
			let mirrored = mirror_mesh(meshes.get(&old_handle)?, axis);
			let h = meshes.add(mirrored);
			remap.insert(old_handle.id(), h.clone());
			h
		};
		if let Some(mut mesh3d) = cloned.world.get_mut::<Mesh3d>(entity) {
			*mesh3d = Mesh3d(new_handle);
		}
	}

	Some(cloned)
}

/// Memoized [`Handle<WorldAsset>`]s keyed by [`SceneRef`] (path + mirror).
#[derive(Resource, Default)]
pub struct SceneRefHandles {
	cache: HashMap<SceneRef, Handle<WorldAsset>>,
}

impl SceneRefHandles {
	fn ensure_unmirrored(
		&mut self,
		scene_ref: &SceneRef,
		asset_server: &AssetServer,
	) -> Handle<WorldAsset> {
		debug_assert!(scene_ref.mirror.is_none());
		if let Some(handle) = self.cache.get(scene_ref) {
			return handle.clone();
		}
		let handle = scene_ref.load_source(asset_server);
		self.cache.insert(scene_ref.clone(), handle.clone());
		handle
	}

	/// Return a strong handle for an **unmirrored** `scene_ref`, loading once per path.
	///
	/// Mirrored refs must go through [`Self::try_resolve`] (or [`SceneRefRoot`] fulfill).
	pub fn handle(
		&mut self,
		scene_ref: &SceneRef,
		asset_server: &AssetServer,
	) -> Handle<WorldAsset> {
		assert!(
			scene_ref.mirror.is_none(),
			"SceneRefHandles::handle is for unmirrored refs; use try_resolve for mirrors"
		);
		self.ensure_unmirrored(scene_ref, asset_server)
	}

	/// Resolve `scene_ref` to a cached handle when ready.
	///
	/// Unmirrored refs always return a (possibly still-loading) handle.
	/// Mirrored refs return [`None`] until the source is
	/// [`AssetServer::is_loaded_with_dependencies`] and the rebuilt world is cached.
	pub fn try_resolve(
		&mut self,
		scene_ref: &SceneRef,
		asset_server: &AssetServer,
		world_assets: &mut Assets<WorldAsset>,
		meshes: &mut Assets<Mesh>,
		type_registry: &AppTypeRegistry,
	) -> Option<Handle<WorldAsset>> {
		if let Some(handle) = self.cache.get(scene_ref) {
			return Some(handle.clone());
		}

		match scene_ref.mirror {
			None => Some(self.ensure_unmirrored(scene_ref, asset_server)),
			Some(axis) => {
				let source_handle =
					self.ensure_unmirrored(&scene_ref.without_mirror(), asset_server);
				if !asset_server.is_loaded_with_dependencies(&source_handle) {
					return None;
				}
				let source = world_assets.get(&source_handle)?;
				let mirrored = mirror_world_asset(source, axis, meshes, type_registry)?;
				let handle = world_assets.add(mirrored);
				self.cache.insert(scene_ref.clone(), handle.clone());
				Some(handle)
			}
		}
	}

	/// Preload many refs (e.g. at startup) so later scene spawns hit the cache.
	///
	/// Mirrored refs only kick off their source load; the mirrored rebuild still
	/// needs [`Self::try_resolve`] once dependencies are ready.
	pub fn preload<'a>(
		&mut self,
		scene_refs: impl IntoIterator<Item = &'a SceneRef>,
		asset_server: &AssetServer,
	) {
		for scene_ref in scene_refs {
			let _ = self.ensure_unmirrored(&scene_ref.without_mirror(), asset_server);
		}
	}

	/// Number of cached handles (source and mirrored).
	pub fn len(&self) -> usize {
		self.cache.len()
	}

	pub fn is_empty(&self) -> bool {
		self.cache.is_empty()
	}
}

fn fulfill_scene_ref_roots(
	mut commands: Commands,
	query: Query<(Entity, &SceneRefRoot), Without<WorldAssetRoot>>,
	mut handles: ResMut<SceneRefHandles>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	type_registry: Res<AppTypeRegistry>,
) {
	for (entity, root) in &query {
		if let Some(handle) = handles.try_resolve(
			&root.0,
			&asset_server,
			&mut world_assets,
			&mut meshes,
			&type_registry,
		) {
			commands.entity(entity).insert(WorldAssetRoot(handle));
		}
	}
}

/// Installs [`SceneRefHandles`] and fulfills [`SceneRefRoot`] → [`WorldAssetRoot`].
pub struct SceneRefPlugin;

impl Plugin for SceneRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<SceneRefHandles>()
			.add_systems(Update, fulfill_scene_ref_roots);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};

	#[test]
	fn unlabeled_glb_gets_scene0() -> anyhow::Result<()> {
		let m = SceneRef::glb("urban/foo.glb");
		assert_eq!(m.labeled_path(), "urban/foo.glb#Scene0");
		Ok(())
	}

	#[test]
	fn labeled_path_preserved() -> anyhow::Result<()> {
		let m = SceneRef::glb("urban/foo.glb#Scene1");
		assert_eq!(m.labeled_path(), "urban/foo.glb#Scene1");
		Ok(())
	}

	#[test]
	fn mirror_changes_cache_key() -> anyhow::Result<()> {
		let base = SceneRef::glb("urban/foo.glb");
		let mirrored = base.clone().mirrored(MirrorAxis::X);
		assert_ne!(base, mirrored);
		assert_eq!(base.labeled_path(), mirrored.labeled_path());
		assert_eq!(mirrored.mirror, Some(MirrorAxis::X));
		Ok(())
	}

	#[test]
	fn mirror_mesh_flips_axis_and_reverses_winding() -> anyhow::Result<()> {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_NORMAL,
			vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(Indices::U32(vec![0, 1, 2]));

		let mirrored = mirror_mesh(&mesh, MirrorAxis::X);

		let Some(VertexAttributeValues::Float32x3(positions)) =
			mirrored.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!((positions[0][0] - 0.0).abs() < 1e-5);
		assert!((positions[1][0] - (-1.0)).abs() < 1e-5);
		assert!((positions[2][0] - 0.0).abs() < 1e-5);

		let Some(VertexAttributeValues::Float32x3(normals)) =
			mirrored.attribute(Mesh::ATTRIBUTE_NORMAL)
		else {
			anyhow::bail!("expected normals");
		};
		// Uniform +Z normals are unchanged by X-scale (scale_recip.z = 1).
		assert!((normals[0][2] - 1.0).abs() < 1e-5);

		match mirrored.indices() {
			Some(Indices::U32(idx)) => assert_eq!(idx.as_slice(), &[0, 2, 1]),
			other => anyhow::bail!("unexpected indices: {other:?}"),
		}
		Ok(())
	}
}
