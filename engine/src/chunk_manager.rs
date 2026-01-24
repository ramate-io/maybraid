use crate::cascade::{Cascade, CascadeChunk, ConstantResolutionMap};
use crate::chunk::{ChunkConfig, LoadedChunks, TerrainChunk, Vec3Key};
use crate::cpu::CpuMeshGenerator;
use crate::shaders::outline::EdgeMaterial;
use bevy::prelude::*;
use rayon::prelude::*;
use sdf::Sdf;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::Arc;

/// Configuration for chunk resolution
#[derive(Resource, Clone, Copy)]
pub struct ChunkResolutionConfig<S: Sdf + Send + Sync> {
	/// Full resolution vertices per chunk side (as power of 2)
	pub base_res_2: u8,
	/// Marker for the SDF that defines the chunk boundaries
	pub sdf: PhantomData<S>,
}

impl<S: Sdf + Send + Sync> Default for ChunkResolutionConfig<S> {
	fn default() -> Self {
		Self { base_res_2: 7, sdf: PhantomData } // 128x128x128 voxels per chunk at full resolution
	}
}

/// Resource wrapper for SDF that can be shared across threads
/// Generic over SDF type to allow different layers at render time
#[derive(Resource)]
pub struct SdfResource<S: Sdf + Send + Sync> {
	pub sdf: Arc<S>,
}

impl<S: Sdf + Send + Sync> SdfResource<S> {
	/// Create from a concrete SDF type
	pub fn new(sdf: S) -> Self {
		Self { sdf: Arc::new(sdf) }
	}

	/// Create from an Arc of a concrete SDF type
	pub fn from_arc(sdf: Arc<S>) -> Self {
		Self { sdf }
	}
}

/// Helper function to wrap a Vec3 coordinate within world bounds
/// If world_size is 0, returns the coordinate unchanged (no wrapping)
fn wrap_coordinate(pos: Vec3, world_size: f32) -> Vec3 {
	if world_size <= 0.0 {
		return pos;
	}
	Vec3::new(
		((pos.x % world_size) + world_size) % world_size,
		((pos.y % world_size) + world_size) % world_size,
		((pos.z % world_size) + world_size) % world_size,
	)
}

/// System that manages chunk loading and unloading based on camera position
/// Generic over SDF type to allow different layers at render time
pub fn manage_chunks<S: Sdf + Send + Sync + 'static>(
	mut commands: Commands,
	camera_query: Query<&Transform, With<Camera3d>>,
	chunk_query: Query<(Entity, &TerrainChunk)>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<EdgeMaterial>>,
	chunk_config: Res<ChunkConfig<S>>,
	resolution_config: Res<ChunkResolutionConfig<S>>,
	sdf_resource: Res<SdfResource<S>>,
	mut loaded_chunks: ResMut<LoadedChunks>,
) {
	let Ok(camera_transform) = camera_query.single() else {
		return;
	};

	let camera_pos = camera_transform.translation;

	// Create cascade instance
	let cascade = Cascade {
		min_size: chunk_config.min_size,
		number_of_rings: chunk_config.number_of_rings as u8,
		resolution_map: ConstantResolutionMap { res_2: resolution_config.base_res_2 },
		grid_radius: chunk_config.grid_radius,
		grid_multiple_2: chunk_config.grid_multiple_2,
	};

	// Get chunks from cascade (separate cascade and grid)
	let cascade_output = match cascade.chunks(camera_pos) {
		Ok(chunks) => chunks,
		Err(e) => {
			log::error!("Failed to get cascade chunks: {}", e);
			return;
		}
	};

	let cascade_chunks = cascade_output.cascade();
	let grid_chunks = cascade_output.grid();

	// Combine for lookup set
	let all_chunks: Vec<_> = cascade_chunks.iter().chain(grid_chunks.iter()).collect();

	// Create set of chunk origins for quick lookup (with wrapping)
	let chunks_to_load_set: HashSet<Vec3Key> = all_chunks
		.iter()
		.map(|chunk| {
			let wrapped_origin = if chunk_config.world_size > 0.0 {
				wrap_coordinate(chunk.origin, chunk_config.world_size)
			} else {
				chunk.origin
			};
			Vec3Key(wrapped_origin)
		})
		.collect();

	// Helper to wrap a chunk origin
	let wrap_chunk_origin = |origin: Vec3| -> Vec3 {
		if chunk_config.world_size > 0.0 {
			wrap_coordinate(origin, chunk_config.world_size)
		} else {
			origin
		}
	};

	// Check existing chunks for unloading
	let mut chunks_to_unload = Vec::new();
	for (entity, chunk) in chunk_query.iter() {
		let wrapped_origin = wrap_chunk_origin(chunk.chunk.origin);
		if !chunks_to_load_set.contains(&Vec3Key(wrapped_origin)) {
			chunks_to_unload.push((entity, chunk.chunk.origin));
		}
	}

	// Unload chunks that are too far away
	for (entity, origin) in chunks_to_unload {
		commands.entity(entity).despawn();
		loaded_chunks.mark_unloaded(&wrap_chunk_origin(origin));
		log::debug!("Unloaded chunk at {:?}", origin);
	}

	// Load new chunks from cascade - process cascade and grid separately
	// Helper to collect chunks that need to be loaded
	let collect_chunks_to_load = |chunks: &[CascadeChunk]| -> Vec<(CascadeChunk, Vec3)> {
		chunks
			.iter()
			.filter_map(|cascade_chunk| {
				let wrapped_origin = wrap_chunk_origin(cascade_chunk.origin);
				if !loaded_chunks.is_loaded(&wrapped_origin) {
					Some((*cascade_chunk, wrapped_origin))
				} else {
					None
				}
			})
			.collect()
	};

	let cascade_chunks_to_generate = collect_chunks_to_load(&cascade_chunks);
	let grid_chunks_to_generate = collect_chunks_to_load(&grid_chunks);

	// Generate meshes in parallel using CPU
	let start_time = std::time::Instant::now();
	let sdf_clone = Arc::clone(&sdf_resource.sdf);

	// Process cascade chunks
	let cascade_mesh_results: Vec<_> = cascade_chunks_to_generate
		.par_iter()
		.map(|(cascade_chunk, _)| {
			let mesh = CpuMeshGenerator::generate_chunk_mesh(cascade_chunk, Arc::clone(&sdf_clone));
			(*cascade_chunk, mesh, true) // true = is_cascade
		})
		.collect();

	// Process grid chunks
	let grid_mesh_results: Vec<_> = grid_chunks_to_generate
		.par_iter()
		.map(|(cascade_chunk, _)| {
			let mesh = CpuMeshGenerator::generate_chunk_mesh(cascade_chunk, Arc::clone(&sdf_clone));
			(*cascade_chunk, mesh, false) // false = is_grid
		})
		.collect();

	// Spawn cascade chunks
	for (cascade_chunk, mesh_opt, _) in cascade_mesh_results {
		let wrapped_origin = wrap_chunk_origin(cascade_chunk.origin);
		if let Some(mesh) = mesh_opt {
			CpuMeshGenerator::spawn_chunk_with_mesh(
				&sdf_resource.sdf,
				&mut commands,
				&mut meshes,
				&mut materials,
				cascade_chunk,
				mesh,
				true, // is_cascade = true
			);
			loaded_chunks.mark_loaded(wrapped_origin);
		} else {
			log::debug!(
				"Skipping cascade chunk at origin {:?} - entirely above terrain",
				cascade_chunk.origin
			);
			loaded_chunks.mark_loaded(wrapped_origin);
		}
	}

	// Spawn grid chunks
	for (cascade_chunk, mesh_opt, _) in grid_mesh_results {
		let wrapped_origin = wrap_chunk_origin(cascade_chunk.origin);
		if let Some(mesh) = mesh_opt {
			CpuMeshGenerator::spawn_chunk_with_mesh(
				&sdf_resource.sdf,
				&mut commands,
				&mut meshes,
				&mut materials,
				cascade_chunk,
				mesh,
				false, // is_cascade = false (is grid)
			);
			loaded_chunks.mark_loaded(wrapped_origin);
		} else {
			log::debug!(
				"Skipping grid chunk at origin {:?} - entirely above terrain",
				cascade_chunk.origin
			);
			loaded_chunks.mark_loaded(wrapped_origin);
		}
	}

	let end_time = std::time::Instant::now();
	let _duration = end_time.duration_since(start_time);
}
