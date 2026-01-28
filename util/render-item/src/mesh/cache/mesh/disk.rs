use crate::mesh::IdentifiedMesh;
use bevy::mesh::SerializedMesh;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use std::fmt::Debug;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use bevy::mesh::Mesh;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct DiskMeshCache<T: Clone + IdentifiedMesh> {
	max_cached_meshes: usize,
	cache_dir: PathBuf,
	__marker: std::marker::PhantomData<T>,
}

impl<T: Clone + IdentifiedMesh> DiskMeshCache<T> {
	const DEFAULT_MAX_CACHED_MESHES: usize = 100;
	const DEFAULT_CACHE_DIR: &str = ".maybraid/mesh-cache";

	pub fn try_new(cache_dir: PathBuf, max_cached_meshes: usize) -> Result<Self, std::io::Error> {
		// create the cache directory if it doesn't exist
		fs::create_dir_all(&cache_dir)?;

		// check if the cache directory is writable
		if !fs::metadata(&cache_dir).unwrap().permissions().mode() & 0o777 == 0 {
			return Err(std::io::Error::new(
				std::io::ErrorKind::PermissionDenied,
				"Cache directory is not writable",
			));
		}

		Ok(Self { cache_dir, max_cached_meshes, __marker: std::marker::PhantomData })
	}

	pub fn try_default() -> Result<Self, std::io::Error> {
		Self::try_new(PathBuf::from(Self::DEFAULT_CACHE_DIR), Self::DEFAULT_MAX_CACHED_MESHES)
	}

	pub fn base_filename_for_cascade_chunk(
		&self,
		identified_mesh: &T,
		cascade_chunk: &CascadeChunk,
	) -> String {
		let name_string = format!("{:?}_{:?}", identified_mesh.id(), cascade_chunk);
		let mut hasher = DefaultHasher::new();
		name_string.hash(&mut hasher);
		let hash = hasher.finish();
		format!("{:x}", hash)
	}

	pub fn value_string_for_cascade_chunk(
		&self,
		identified_mesh: &T,
		cascade_chunk: &CascadeChunk,
	) -> String {
		format!("{:?}_{:?}", identified_mesh.id(), cascade_chunk).to_string()
	}

	pub fn mesh_filename_for_cascade_chunk(
		&self,
		identified_mesh: &T,
		cascade_chunk: &CascadeChunk,
	) -> String {
		format!("{}.mesh", self.base_filename_for_cascade_chunk(identified_mesh, cascade_chunk),)
	}

	pub fn value_filename_for_cascade_chunk_mesh(
		&self,
		identified_mesh: &T,
		cascade_chunk: &CascadeChunk,
	) -> String {
		format!("{}.value", self.base_filename_for_cascade_chunk(identified_mesh, cascade_chunk))
			.to_string()
	}

	pub fn path_for_cascade_chunk_mesh(
		&self,
		identified_mesh: &T,
		cascade_chunk: &CascadeChunk,
	) -> PathBuf {
		let version = env!("CARGO_PKG_VERSION");
		self.cache_dir
			.join(version)
			.join(std::any::type_name::<T>())
			.join(self.mesh_filename_for_cascade_chunk(identified_mesh, cascade_chunk))
	}

	pub fn path_for_cascade_chunk_value(
		&self,
		identified_mesh: &T,
		cascade_chunk: &CascadeChunk,
	) -> PathBuf {
		let version = env!("CARGO_PKG_VERSION");
		self.cache_dir
			.join(version)
			.join(std::any::type_name::<T>())
			.join(self.value_filename_for_cascade_chunk_mesh(identified_mesh, cascade_chunk))
	}

	pub fn save_mesh(&self, identified_mesh: &T, mesh: &Mesh, cascade_chunk: &CascadeChunk) {
		// write the mesh to the cache
		let cache_path = self.path_for_cascade_chunk_mesh(identified_mesh, cascade_chunk);

		if let Some(parent) = cache_path.parent() {
			if let Err(err) = fs::create_dir_all(parent) {
				log::warn!("Failed to create mesh cache directory {:?}: {}", parent, err);
				return;
			}
		}

		let serialized = SerializedMesh::from_mesh(mesh.clone());

		let raw = match bincode::serialize(&serialized) {
			Ok(bytes) => bytes,
			Err(_) => return, // cache-only; fail silently
		};

		let compressed = compress_prepend_size(&raw);

		if let Err(err) = fs::write(&cache_path, compressed) {
			log::warn!("Failed to write mesh cache {:?}: {}", cache_path, err);
		}

		// write the value to the cache
		let value = self.value_string_for_cascade_chunk(identified_mesh, cascade_chunk);
		let cache_path = self.path_for_cascade_chunk_value(identified_mesh, cascade_chunk);

		if let Err(err) = fs::write(&cache_path, value) {
			log::warn!("Failed to write value cache {:?}: {}", cache_path, err);
		}
	}

	pub fn load_mesh(&self, identified_mesh: &T, cascade_chunk: &CascadeChunk) -> Option<Mesh> {
		// check if the value matches the expected value, this is a Hash + Eq implementation for the CascadeChunk
		let expected_value = self.value_string_for_cascade_chunk(identified_mesh, cascade_chunk);
		let actual_value =
			fs::read_to_string(self.path_for_cascade_chunk_value(identified_mesh, cascade_chunk))
				.ok()?;
		if expected_value != actual_value {
			return None;
		}

		// load the mesh from the cache
		let cache_path = self.path_for_cascade_chunk_mesh(identified_mesh, cascade_chunk);

		let compressed = fs::read(cache_path).ok()?;
		let raw = decompress_size_prepended(&compressed).ok()?;

		let serialized: SerializedMesh = bincode::deserialize(&raw).ok()?;
		Some(serialized.into_mesh())
	}

	pub fn evict_oldest_cached_meshes(&self) {
		let cache_root = self.cache_dir.join(std::any::type_name::<T>());
		let entries = match fs::read_dir(&cache_root) {
			Ok(e) => e,
			Err(_) => return,
		};

		let mut files: Vec<(PathBuf, SystemTime)> = entries
			.filter_map(|entry| {
				let entry = entry.ok()?;
				let meta = entry.metadata().ok()?;
				let modified = meta.modified().ok()?;
				Some((entry.path(), modified))
			})
			.collect();

		if files.len() <= self.max_cached_meshes {
			return;
		}

		// Oldest first
		files.sort_by_key(|(_, time)| *time);

		let to_remove = files.len().saturating_sub(self.max_cached_meshes);
		for (path, _) in files.into_iter().take(to_remove) {
			let _ = fs::remove_file(&path);
		}
	}
}
