use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, SerializedMesh, VertexAttributeValues};
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use sdf::Sdf;

use crate::sdf::cpu_shot::{CpuShotOptions, CpuShotSdf, WallFaces};

// Used only by the ignored, serial comparison. Normal tests leave it disabled.
struct CountingAllocator;
static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static REALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static REQUESTED: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let pointer = unsafe { System.alloc(layout) };
		if TRACK.load(Ordering::Relaxed) && !pointer.is_null() {
			ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
			REQUESTED.fetch_add(layout.size() as u64, Ordering::Relaxed);
		}
		pointer
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		let pointer = unsafe { System.alloc_zeroed(layout) };
		if TRACK.load(Ordering::Relaxed) && !pointer.is_null() {
			ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
			REQUESTED.fetch_add(layout.size() as u64, Ordering::Relaxed);
		}
		pointer
	}

	unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
		unsafe { System.dealloc(pointer, layout) };
	}

	unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
		let resized = unsafe { System.realloc(pointer, layout, size) };
		if TRACK.load(Ordering::Relaxed) && !resized.is_null() {
			REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
			REQUESTED.fetch_add(size as u64, Ordering::Relaxed);
		}
		resized
	}
}

#[derive(Clone)]
struct Ground {
	stepped: bool,
}

impl Sdf for Ground {
	fn distance(&self, point: Vec3) -> f32 {
		let step = if self.stepped && point.x > 12.0 { 0.7 } else { 0.0 };
		point.y - (0.4 + point.x * 0.12 - point.z * 0.05 + step)
	}
}

fn chunk(res_2: u8) -> CascadeChunk {
	CascadeChunk {
		world: 0,
		origin: Vec3::new(10.0, -2.0, -3.0),
		size: 6.0,
		extent: Some(Vec3::new(5.0, 6.0, 3.0)),
		res_2,
		omit: None,
	}
}

fn mesh_bytes(mesh: &Mesh) -> anyhow::Result<(usize, usize)> {
	let mut logical = 0;
	let mut capacity = 0;
	for (_, values) in mesh.attributes() {
		logical += values.get_bytes().len();
		capacity += match values {
			VertexAttributeValues::Float32x3(values) => values.capacity() * size_of::<[f32; 3]>(),
			VertexAttributeValues::Float32x2(values) => values.capacity() * size_of::<[f32; 2]>(),
			_ => anyhow::bail!("unexpected CpuShot attribute"),
		};
	}
	match mesh.indices() {
		Some(Indices::U32(values)) => {
			logical += values.len() * size_of::<u32>();
			capacity += values.capacity() * size_of::<u32>();
		}
		_ => anyhow::bail!("missing CpuShot U32 indices"),
	}
	Ok((logical, capacity))
}

fn assert_geometry_equal(legacy: &Mesh, compact: &Mesh) -> anyhow::Result<()> {
	assert_eq!(legacy.primitive_topology(), compact.primitive_topology());
	assert_eq!(legacy.asset_usage, compact.asset_usage);
	let old = bincode::serialize(&SerializedMesh::from_mesh(legacy.clone()))?;
	let new = bincode::serialize(&SerializedMesh::from_mesh(compact.clone()))?;
	assert_eq!(old, new, "all serialized attributes and indices must match bit for bit");
	Ok(())
}

#[test]
fn compact_outputs_match_legacy_geometry_for_planes_steps_and_selected_walls() -> anyhow::Result<()>
{
	let mut legacy_slack = 0;
	for stepped in [false, true] {
		for wall_faces in [
			WallFaces::NONE,
			WallFaces { neg_x: true, pos_z: true, ..WallFaces::NONE },
			WallFaces::ALL,
		] {
			let sdf = Ground { stepped };
			let options = CpuShotOptions { wall_faces };
			let legacy = options
				.build_mesh(&sdf, &chunk(3), true)
				.ok_or_else(|| anyhow::anyhow!("legacy fixture mesh missing"))?;
			let compact = sdf
				.cpu_chunk_mesh(&chunk(3), &options)
				.ok_or_else(|| anyhow::anyhow!("compact fixture mesh missing"))?;
			assert_geometry_equal(&legacy, &compact)?;
			let (old_logical, old_capacity) = mesh_bytes(&legacy)?;
			let (logical, capacity) = mesh_bytes(&compact)?;
			assert_eq!(old_logical, logical);
			assert_eq!(capacity, logical, "published Vecs must not retain spare element capacity");
			legacy_slack += old_capacity - old_logical;
			let vertices = compact
				.attribute(Mesh::ATTRIBUTE_POSITION)
				.and_then(VertexAttributeValues::as_float3)
				.ok_or_else(|| anyhow::anyhow!("positions missing"))?;
			let indices = compact.indices().ok_or_else(|| anyhow::anyhow!("indices missing"))?;
			assert_eq!(indices.len() % 3, 0);
			assert!(indices.iter().all(|index| index < vertices.len()));
			assert!(vertices.iter().all(|p| p.iter().all(|v| v.is_finite())));
			if !stepped && !wall_faces.any() {
				for vertex in vertices {
					assert!(sdf.distance(Vec3::from(*vertex) + chunk(3).origin).abs() < 1e-5);
				}
			}
		}
	}
	assert!(legacy_slack > 0, "fixtures must exercise retained growth capacity");
	Ok(())
}

#[test]
fn disk_payload_roundtrip_preserves_geometry_and_decodes_without_spare_capacity(
) -> anyhow::Result<()> {
	let options = CpuShotOptions { wall_faces: WallFaces::ALL };
	for legacy in [true, false] {
		let mesh = options
			.build_mesh(&Ground { stepped: true }, &chunk(4), legacy)
			.ok_or_else(|| anyhow::anyhow!("fixture mesh missing"))?;
		let serialized = bincode::serialize(&SerializedMesh::from_mesh(mesh.clone()))?;
		let decoded: SerializedMesh = bincode::deserialize(&serialized)?;
		let decoded = decoded.into_mesh();
		assert_geometry_equal(&mesh, &decoded)?;
		assert_eq!(
			decoded.asset_usage,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
		);
		let (logical, capacity) = mesh_bytes(&decoded)?;
		assert_eq!(
			logical, capacity,
			"warm disk decode already uses initialized lengths in this fixture"
		);
	}
	Ok(())
}

#[test]
#[ignore = "focused allocation/time comparison; run alone with --ignored --nocapture --test-threads=1"]
fn capacity_cost_comparison() -> anyhow::Result<()> {
	anyhow::ensure!(
		std::env::var("RAYON_NUM_THREADS").as_deref() == Ok("1"),
		"run with RAYON_NUM_THREADS=1"
	);
	// Warm each case first. Timing and allocation passes are separate; timing
	// leaves counting disabled. Requested bytes measure allocation traffic, not
	// retained bytes, physical allocator release, or RSS.
	let sdf = Ground { stepped: true };
	for res_2 in [4, 5] {
		let fixture = chunk(res_2);
		for wall_faces in [WallFaces::NONE, WallFaces::ALL] {
			let options = CpuShotOptions { wall_faces };
			for legacy in [true, false] {
				let _ = options.build_mesh(&sdf, &fixture, legacy);
			}
			for iteration in 0..30 {
				for legacy in if iteration % 2 == 0 { [true, false] } else { [false, true] } {
					let started = Instant::now();
					let mesh = options.build_mesh(&sdf, &fixture, legacy);
					let nanos = started.elapsed().as_nanos();
					let mesh = mesh.ok_or_else(|| anyhow::anyhow!("comparison mesh missing"))?;
					let (logical, capacity) = mesh_bytes(&mesh)?;
					println!("cpu_shot_capacity pass=timing iteration={iteration} res_2={res_2} legacy={legacy} walls={} nanos={nanos} logical_bytes={logical} capacity_bytes={capacity}", wall_faces.any());
				}
			}
			for iteration in 0..3 {
				for legacy in if iteration % 2 == 0 { [true, false] } else { [false, true] } {
					ALLOCATIONS.store(0, Ordering::Relaxed);
					REALLOCATIONS.store(0, Ordering::Relaxed);
					REQUESTED.store(0, Ordering::Relaxed);
					TRACK.store(true, Ordering::Relaxed);
					let mesh = options.build_mesh(&sdf, &fixture, legacy);
					TRACK.store(false, Ordering::Relaxed);
					let allocations = ALLOCATIONS.load(Ordering::Relaxed);
					let reallocations = REALLOCATIONS.load(Ordering::Relaxed);
					let requested = REQUESTED.load(Ordering::Relaxed);
					let mesh = mesh.ok_or_else(|| anyhow::anyhow!("comparison mesh missing"))?;
					let (logical, capacity) = mesh_bytes(&mesh)?;
					println!("cpu_shot_capacity pass=allocation iteration={iteration} res_2={res_2} legacy={legacy} walls={} allocations={allocations} reallocations={reallocations} requested_bytes={requested} logical_bytes={logical} capacity_bytes={capacity}", wall_faces.any());
				}
			}
		}
	}
	Ok(())
}
