//! Geometry containment, not agreement with structural indexing metadata.
use crate::packed::{
	cache::TilePayload,
	instances::{PackedBatch, PackedBatchKey, PackedInstance},
	kit::{KitState, PackedKitMeshes},
};
use crate::{ChicoGroveHost, ForestGroveKind, ForestLayer};
use bevy::{
	asset::RenderAssetUsages,
	mesh::{PrimitiveTopology, VertexAttributeValues},
	prelude::*,
	shape::Aabb3d,
};
use lod::VisualMaterialKind;
use std::{collections::HashMap, sync::Arc};
use vegetation_groves::{FlatTerrainSample, GroveExtent};

fn payload(elevation: f32) -> Result<TilePayload, String> {
	let tile = ForestGroveKind::RiparianGeneral.grow_tile(
		GroveExtent::new(Vec3::ZERO, Vec3::new(80., 1., 80.)),
		&FlatTerrainSample { elevation, steepness: 0.1 },
	);
	TilePayload::from_host(&ChicoGroveHost::new(tile, ForestLayer::UpperCanopy), 1, true)
		.map_err(|reason| format!("{reason:?}"))
}
fn mesh() -> Mesh {
	Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[-1., -1., -1.], [1., 1., 1.], [-1., 1., -1.]],
		)
		.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3])
}
struct Kits {
	cache: PackedKitMeshes,
	vertices: HashMap<&'static str, Vec<Vec3>>,
	_assets: Assets<Mesh>,
}
impl Kits {
	fn for_payload(payload: &TilePayload) -> Result<Self, String> {
		let mut cache = PackedKitMeshes::default();
		let mut assets = Assets::<Mesh>::default();
		let mut vertices = HashMap::new();
		for batch in payload.lods.iter().flat_map(|batches| batches.iter()) {
			if cache.states.contains_key(batch.key.kit) {
				continue;
			}
			let mesh = mesh();
			assert!(PackedKitMeshes::supports_mesh(&mesh));
			let Some(VertexAttributeValues::Float32x3(points)) =
				mesh.attribute(Mesh::ATTRIBUTE_POSITION)
			else {
				return Err("fixture positions missing".into());
			};
			vertices.insert(batch.key.kit, points.iter().copied().map(Vec3::from_array).collect());
			let bounds =
				PackedKitMeshes::bounds_for_mesh(&mesh).ok_or("fixture mesh Aabb missing")?;
			cache.states.insert(batch.key.kit, KitState::Ready(assets.add(mesh)));
			cache.local_bounds.insert(batch.key.kit, bounds);
		}
		Ok(Self { cache, vertices, _assets: assets })
	}
}
fn contains(bounds: Aabb3d, p: Vec3) -> bool {
	let min = Vec3::from(bounds.min) - Vec3::splat(1e-4);
	let max = Vec3::from(bounds.max) + Vec3::splat(1e-4);
	p.cmpge(min).all() && p.cmple(max).all()
}
#[test]
fn elevated_and_depressed_groves_bound_every_actual_kit_vertex() -> Result<(), String> {
	for elevation in [-300., 300.] {
		let mut payload = payload(elevation)?;
		assert!(payload.plant_count > 0);
		let structural = (payload.structural_lod, payload.center, payload.radius);
		let old = payload.bounds;
		let kits = Kits::for_payload(&payload)?;
		let bounds = payload.geometric_bounds(&kits.cache).ok_or("geometric bounds missing")?;
		let mut old_misses = 0;
		let mut vertices = 0;
		for batch in payload.lods.iter().flat_map(|batches| batches.iter()) {
			for instance in batch.instances.iter() {
				for &local in
					kits.vertices.get(batch.key.kit).ok_or("missing actual mesh vertices")?
				{
					let point = instance.world_from_local.transform_point3(local);
					old_misses += usize::from(!contains(old, point));
					assert!(
						contains(bounds, point),
						"elevation{elevation}: {point:?} escaped {bounds:?}"
					);
					vertices += 1;
				}
			}
		}
		println!(
			"elevation={elevation} checked_vertices={vertices} old_index_bounds_misses={old_misses} old={old:?} geometric={bounds:?}"
		);
		assert!(
			vertices > 0 && old_misses > 0,
			"old indexing footprint must miss terrain-displaced geometry"
		);
		payload.bounds = bounds;
		assert_eq!((payload.structural_lod, payload.center, payload.radius), structural);
		let mut missing = kits.cache;
		let kit = *missing.local_bounds.keys().next().ok_or("no kits")?;
		missing.local_bounds.remove(kit);
		assert!(
			payload.geometric_bounds(&missing).is_none(),
			"never substitute indexing bounds for unavailable geometry"
		);
	}
	Ok(())
}
#[test]
fn merged_nonuniform_leaf_padding_uses_shader_root_scale() -> Result<(), String> {
	let mut payload = payload(0.)?;
	let root = Mat4::from_scale_rotation_translation(
		Vec3::new(10., 1., 1.),
		Quat::IDENTITY,
		Vec3::new(150., -300., -350.),
	);
	let model = root
		* Mat4::from_rotation_y(std::f32::consts::FRAC_PI_4)
		* Mat4::from_scale(Vec3::new(1., 1., 0.1));
	let instance = PackedInstance {
		world_from_local: model,
		plant_origin: Vec3::ZERO,
		collection_root: root,
		part_scale: 1.,
		merged_leaf: true,
	};
	payload.lods = std::array::from_fn(|_| Arc::from([]));
	payload.lods[0] = Arc::from([PackedBatch {
		key: PackedBatchKey {
			kit: "test",
			material_kind: VisualMaterialKind::Leaf,
			material_hash: 0,
		},
		base_color: Vec4::ONE,
		instances: Arc::from([instance]),
	}]);
	let kits = Kits::for_payload(&payload)?;
	let actual = payload.geometric_bounds(&kits.cache).ok_or("missing leaf bounds")?;
	let unexpanded_min = (0..8)
		.map(|mask| {
			Vec3::new(
				if mask & 1 == 0 { -1. } else { 1. },
				if mask & 2 == 0 { -1. } else { 1. },
				if mask & 4 == 0 { -1. } else { 1. },
			)
		})
		.map(|v| model.transform_point3(v))
		.fold(Vec3::splat(f32::INFINITY), Vec3::min);
	let model_scale = model
		.x_axis
		.truncate()
		.length()
		.max(model.y_axis.truncate().length())
		.max(model.z_axis.truncate().length());
	assert!(model_scale < 10.);
	let expected_padding = 0.108 * 10.;
	for axis in 0..3 {
		assert!(
			(unexpanded_min[axis] - Vec3::from(actual.min)[axis] - expected_padding).abs() < 1e-4
		);
	}
	Ok(())
}
#[test]
fn invalid_kit_positions_and_procedural_shadow_revisions_do_not_keep_stale_bounds()
-> Result<(), String> {
	let empty = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
		.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
	assert!(PackedKitMeshes::bounds_for_mesh(&empty).is_none());
	let invalid =
		mesh().with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[f32::NAN, 0., 0.]]);
	assert!(PackedKitMeshes::bounds_for_mesh(&invalid).is_none());
	let mut assets = Assets::<Mesh>::default();
	let geometry = mesh();
	let bounds = PackedKitMeshes::bounds_for_mesh(&geometry).ok_or("missing valid bounds")?;
	let handle = assets.add(geometry);
	let mut app = App::new();
	app.init_resource::<PackedKitMeshes>()
		.add_message::<AssetEvent<bevy::gltf::Gltf>>()
		.add_message::<AssetEvent<Mesh>>()
		.add_systems(Update, crate::packed::kit::invalidate_changed_kits);
	let kit = crate::packed::instances::SHADOW_KIT;
	app.world_mut()
		.resource_mut::<PackedKitMeshes>()
		.states
		.insert(kit, KitState::Ready(handle.clone()));
	app.world_mut()
		.resource_mut::<PackedKitMeshes>()
		.local_bounds
		.insert(kit, bounds);
	app.world_mut().write_message(AssetEvent::<Mesh>::Modified { id: handle.id() });
	app.update();
	let cache = app.world().resource::<PackedKitMeshes>();
	assert_eq!(cache.epoch, 1);
	assert!(cache.ready_bounds(kit).is_none());
	assert!(matches!(cache.states.get(kit), Some(KitState::Ready(_))));
	app.world_mut()
		.resource_mut::<PackedKitMeshes>()
		.local_bounds
		.insert(kit, bounds);
	app.world_mut().write_message(AssetEvent::<Mesh>::Removed { id: handle.id() });
	app.update();
	let cache = app.world().resource::<PackedKitMeshes>();
	assert_eq!(cache.epoch, 2);
	assert!(cache.ready_bounds(kit).is_none());
	assert!(matches!(cache.states.get(kit), Some(KitState::Unsupported)));
	Ok(())
}

#[test]
fn native_calculate_bounds_preserves_only_explicit_manual_instance_bounds() -> Result<(), String> {
	use bevy::asset::AssetApp;
	use bevy::camera::{
		primitives::Aabb,
		visibility::{NoAutoAabb, calculate_bounds},
	};
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
		.init_asset::<Mesh>()
		.add_systems(PostUpdate, calculate_bounds.after(bevy::asset::AssetEventSystems));
	let first = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh());
	let custom = Aabb {
		center: Vec3::new(150., -300., -350.).into(),
		half_extents: Vec3::new(40., 25., 40.).into(),
	};
	let packed = app.world_mut().spawn((Mesh3d(first.clone()), custom, NoAutoAabb)).id();
	let control = app.world_mut().spawn((Mesh3d(first), custom)).id();
	app.update();
	assert_eq!(app.world().get::<Aabb>(packed), Some(&custom));
	assert_eq!(
		app.world().get::<Aabb>(control).map(|a| a.center),
		Some(Vec3::ZERO.into()),
		"unmarked initial Mesh3d must reproduce kit-local overwrite"
	);

	let second =
		app.world_mut()
			.resource_mut::<Assets<Mesh>>()
			.add(mesh().with_inserted_attribute(
				Mesh::ATTRIBUTE_POSITION,
				vec![[4., -1., -1.], [8., 1., 1.], [6., 1., -1.]],
			));
	app.world_mut().entity_mut(packed).insert(Mesh3d(second.clone()));
	app.world_mut().entity_mut(control).insert((Mesh3d(second.clone()), custom));
	app.update();
	assert_eq!(app.world().get::<Aabb>(packed), Some(&custom));
	assert_eq!(
		app.world().get::<Aabb>(control).map(|a| a.center),
		Some(Vec3::new(6., 0., 0.).into()),
		"handle changes must preserve manual bounds only"
	);

	app.world_mut().entity_mut(control).insert(custom);
	app.world_mut()
		.resource_mut::<Assets<Mesh>>()
		.get_mut(&second)
		.ok_or("missing modified mesh")?
		.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[10., -1., -1.], [14., 1., 1.], [12., 1., -1.]],
		);
	app.update();
	assert_eq!(app.world().get::<Aabb>(packed), Some(&custom));
	assert_eq!(
		app.world().get::<Aabb>(control).map(|a| a.center),
		Some(Vec3::new(12., 0., 0.).into()),
		"asset changes must preserve manual bounds only"
	);
	Ok(())
}
