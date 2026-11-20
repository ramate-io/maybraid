//! One shared ray contribution for all admitted Combined instances, including grains.
use crate::{
	composition::CombinedQuality,
	parts::{
		combined::{
			assets::{CombinedAsset, Style},
			CombinedState,
		},
		packaged::StaticVolume,
	},
};
use bevy::{
	asset::embedded_asset,
	camera::visibility::NoFrustumCulling,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	render::{
		render_resource::{
			AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
		},
		storage::ShaderBuffer,
	},
	shader::ShaderRef,
};

const GRID: UVec3 = UVec3::new(16, 8, 16);
const CELLS: usize = 16 * 8 * 16;
const MAX_PRIMITIVES: usize = 4 * (15 + 96 + 96)
	+ 4 * (48 + 32 + 24)
	+ crate::parts::herd::MAX_PUFFS
	+ crate::parts::herd::MAX_FINE
	+ crate::parts::herd::MAX_GRAINS;
// Worst case: every primitive in every bin. No truncation or candidate eviction.
pub const MAX_PAYLOAD_BYTES: u64 = ((22
	+ 28 * 4
	+ MAX_PRIMITIVES * 7
	+ (CELLS + 1).div_ceil(4)
	+ CELLS.div_ceil(4)
	+ (MAX_PRIMITIVES * CELLS).div_ceil(4))
	* 16) as u64;
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CombinedMaterial {
	#[texture(0, dimension = "3d")]
	#[sampler(7)]
	pub shoulder: Handle<Image>,
	#[texture(1, dimension = "3d")]
	pub column: Handle<Image>,
	#[texture(2, dimension = "3d")]
	pub swept: Handle<Image>,
	#[texture(3, dimension = "3d")]
	pub billow: Handle<Image>,
	#[texture(4, dimension = "3d")]
	pub curl: Handle<Image>,
	#[texture(5, dimension = "3d")]
	pub trail: Handle<Image>,
	#[texture(6, dimension = "3d")]
	pub cluster: Handle<Image>,
	#[storage(8, read_only)]
	pub payload: Handle<ShaderBuffer>,
	#[texture(9, dimension = "3d")]
	pub field: Handle<Image>,
}
impl Material for CombinedMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/combined/volume.wesl").into()
	}
	fn fragment_shader() -> ShaderRef {
		Self::vertex_shader()
	}
	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Blend
	}
	fn enable_prepass() -> bool {
		false
	}
	fn enable_shadows() -> bool {
		false
	}
	fn specialize(
		_: &MaterialPipeline,
		p: &mut RenderPipelineDescriptor,
		_: &MeshVertexBufferLayoutRef,
		_: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		p.primitive.cull_mode = None;
		if let Some(depth) = p.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
			depth.depth_compare = Some(CompareFunction::Always);
		}
		Ok(())
	}
}
#[derive(Clone, Copy, Debug)]
pub struct Primitive {
	pub inverse: Mat4,
	pub masked: bool,
	pub coefficients: Vec4,
	pub parameters: Vec4,
	pub min: Vec3,
	pub max: Vec3,
}
impl Primitive {
	pub fn placed(mut self, placement: Mat4) -> Self {
		self.inverse *= placement.inverse();
		(self.min, self.max) = Self::transformed_bounds(placement, self.min, self.max);
		self
	}
	pub fn rows(self) -> [Vec4; 7] {
		[
			self.inverse.row(0),
			self.inverse.row(1),
			self.inverse.row(2),
			self.coefficients,
			self.parameters,
			self.min.extend(if self.masked { 1. } else { 0. }),
			self.max.extend(0.),
		]
	}
	pub fn transformed_bounds(model: Mat4, min: Vec3, max: Vec3) -> (Vec3, Vec3) {
		let mut a = Vec3::splat(f32::INFINITY);
		let mut b = -a;
		for i in 0..8 {
			let p = Vec3::new(
				if i & 1 == 0 { min.x } else { max.x },
				if i & 2 == 0 { min.y } else { max.y },
				if i & 4 == 0 { min.z } else { max.z },
			);
			let p = model.transform_point3(p);
			a = a.min(p);
			b = b.max(p);
		}
		(a, b)
	}
}
#[derive(Default)]
pub struct Payload {
	pub values: Vec<Vec4>,
	pub references: usize,
	pub max_bin: usize,
}
impl Payload {
	pub fn build(
		&mut self,
		primitives: &[Primitive],
		volumes: [&StaticVolume; 7],
		style: &Style,
		quality: CombinedQuality,
	) {
		self.values.clear();
		self.references = 0;
		self.max_bin = 0;
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = -min;
		for p in primitives {
			min = min.min(p.min);
			max = max.max(p.max);
		}
		if primitives.is_empty() {
			min = Vec3::ZERO;
			max = Vec3::ONE;
		}
		min -= Vec3::splat(0.12);
		max += Vec3::splat(0.12);
		let extent = (max - min).max(Vec3::splat(0.001));
		max = min + extent;
		self.values.resize(22, Vec4::ZERO);
		self.values[0] = Vec4::new(primitives.len() as f32, 22., 0., 0.);
		self.values[1] = min.extend(if quality == CombinedQuality::Low { 0.06 } else { 0.03 });
		self.values[2] = max.extend(if quality == CombinedQuality::Low { 1. } else { 3. });
		self.values[3] = GRID.as_vec3().extend(0.);
		self.values[4] = Vec3::from_array(style.color_linear_rgb).extend(style.extinction_per_m);
		self.values[5] = Vec4::new(
			style.concentration_variation,
			style.overlap_cap,
			style.ramp_end_density,
			style.offline_light_contrast,
		);
		let cache_size = if quality == CombinedQuality::Low {
			crate::parts::combined::cache::LOW_SIZE
		} else {
			crate::parts::combined::cache::MAX_SIZE
		};
		self.values[6] =
			Vec4::new(512., cache_size.x as f32, cache_size.y as f32, cache_size.z as f32);
		for (i, v) in volumes.iter().enumerate() {
			let lo = Vec3::from_array(v.manifest.bounds.min);
			let hi = Vec3::from_array(v.manifest.bounds.max);
			self.values[7 + 2 * i] = lo.extend(v.manifest.density_scale);
			self.values[8 + 2 * i] = (hi - lo).recip().extend(0.);
		}
		for p in primitives {
			self.values.extend(p.rows());
		}
		let mut bins = vec![Vec::new(); CELLS];
		for (index, p) in primitives.iter().enumerate() {
			let lo = (((p.min - min) / extent) * GRID.as_vec3())
				.floor()
				.max(Vec3::ZERO)
				.as_uvec3()
				.min(GRID - UVec3::ONE);
			let hi = (((p.max - min) / extent) * GRID.as_vec3())
				.floor()
				.max(Vec3::ZERO)
				.as_uvec3()
				.min(GRID - UVec3::ONE);
			for z in lo.z..=hi.z {
				for y in lo.y..=hi.y {
					for x in lo.x..=hi.x {
						bins[(x + GRID.x * (y + GRID.y * z)) as usize].push(index as u32);
					}
				}
			}
		}
		let mut offsets = Vec::with_capacity(CELLS + 1);
		let mut indices = Vec::new();
		offsets.push(0);
		let mut grain_starts = Vec::with_capacity(CELLS);
		for mut bin in bins {
			bin.sort_by_key(|&i| primitives[i as usize].parameters.w > 1.5);
			grain_starts.push(
				(indices.len()
					+ bin
						.iter()
						.take_while(|&&i| primitives[i as usize].parameters.w < 1.5)
						.count()) as u32,
			);
			self.max_bin = self.max_bin.max(bin.len());
			indices.extend(bin);
			offsets.push(indices.len() as u32);
		}
		self.references = indices.len();
		let offsets_base = self.values.len();
		Self::pack(&mut self.values, &offsets);
		let grain_base = self.values.len();
		Self::pack(&mut self.values, &grain_starts);
		self.values[3].w = grain_base as f32;
		let indices_base = self.values.len();
		Self::pack(&mut self.values, &indices);
		self.values[0].z = offsets_base as f32;
		self.values[0].w = indices_base as f32;
	}
	fn pack(out: &mut Vec<Vec4>, values: &[u32]) {
		for chunk in values.chunks(4) {
			let mut a = [0.; 4];
			for (i, &v) in chunk.iter().enumerate() {
				a[i] = v as f32;
			}
			out.push(Vec4::from_array(a));
		}
	}
}
impl Payload {
	pub fn colliders(&mut self, colliders: &[crate::parts::combined::interaction::Interactor]) {
		use crate::parts::combined::interaction::Shape;
		self.values[21] =
			Vec4::new(colliders.len().min(28) as f32, self.values.len() as f32, 0.025, 0.);
		for c in colliders.iter().take(28) {
			let inverse =
				Mat4::from_rotation_translation(c.rotation.as_quat(), c.position.as_vec3())
					.inverse();
			self.values.extend([inverse.row(0), inverse.row(1), inverse.row(2)]);
			self.values.push(match c.shape {
				Shape::Capsule { radius, half_segment } => {
					Vec4::new(radius as f32, half_segment as f32, 0., 0.)
				}
				Shape::Box { half } => half.as_vec3().extend(1.),
			});
		}
	}
}
impl Payload {
	pub fn terrain(&mut self, h: Option<&crate::parts::herd::terrain::Heightfield>) {
		let Some(h) = h else {
			return;
		};
		self.values[21].w = self.values.len() as f32;
		self.values.push(Vec4::new(
			h.origin.x as f32,
			h.origin.y as f32,
			h.step as f32,
			h.dimensions[0] as f32,
		));
		self.values.push(Vec4::new(h.dimensions[1] as f32, 0., 0., 0.));
		self.values.extend(
			h.heights
				.iter()
				.zip(&h.valid)
				.map(|(v, ok)| Vec4::new(*v as f32, if *ok { 1. } else { 0. }, 0., 0.)),
		);
	}
}
const HERD_PAYLOAD_BYTES: u64 = ((22
	+ 20 * 4
	+ 160 * 7
	+ (CELLS + 1).div_ceil(4)
	+ CELLS.div_ceil(4)
	+ (160 * CELLS).div_ceil(4)
	+ 2 + 65 * 65)
	* 16) as u64;

struct Scene {
	field: Handle<Image>,
	entity: Entity,
	_material: Handle<CombinedMaterial>,
	buffer: Handle<ShaderBuffer>,
}
#[derive(Resource, Default)]
pub struct CombinedRenderer {
	signature: Vec<(Entity, u32, u32, u32, Mat4)>,
	generation: u64,
	scene_version: u64,
	scene: Option<Scene>,
	pub payload: Payload,
	pub primitives: usize,
	pub grain_count: usize,
	regions: std::collections::HashMap<(Entity, u8), RegionScene>,
}
impl CombinedRenderer {
	pub fn region_resources(&self) -> (usize, usize, u64, u64) {
		let count = self.regions.len();
		(
			count,
			self.regions
				.values()
				.map(|r| r.payload.values.first().map_or(0, |v| v.x as usize))
				.sum(),
			count as u64 * crate::parts::combined::cache::BYTES,
			self.regions.values().map(|r| r.capacity).sum(),
		)
	}
}
pub fn plugin(app: &mut App) {
	embedded_asset!(app, "volume.wesl");
	crate::parts::combined::cache::plugin(app);
	app.add_plugins(MaterialPlugin::<CombinedMaterial>::default())
		.init_resource::<CombinedRenderer>();
}

struct RegionScene {
	scene: Scene,
	payload: Payload,
	tick: u32,
	revision: u64,
	options: crate::composition::ChoreographyOptions,
	generation: u64,
	capacity: u64,
}
const REGION_PAYLOAD_BYTES: u64 = ((22
	+ 20 * 4
	+ (48 + 32 + 24) * 7
	+ (CELLS + 1).div_ceil(4)
	+ CELLS.div_ceil(4)
	+ ((48 + 32 + 24) * CELLS).div_ceil(4))
	* 16) as u64;
pub fn compose(
	interaction: Option<Res<crate::parts::combined::interaction::InteractionScene>>,
	mut commands: Commands,
	states: Query<(Entity, &CombinedState, &GlobalTransform)>,
	herds: Query<(Entity, &crate::parts::herd::HerdState)>,
	choreography: Query<(
		Entity,
		&crate::parts::cartoon_flow::CartoonFlow,
		&crate::parts::combined_choreography::OpticalChoreography,
	)>,
	mut images: ResMut<Assets<Image>>,
	mut cache_input: ResMut<crate::parts::combined::cache::CacheInput>,
	assets: Res<Assets<CombinedAsset>>,
	volumes: Res<Assets<StaticVolume>>,
	mut renderer: ResMut<CombinedRenderer>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<CombinedMaterial>>,
	mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
	let mut active = states.iter().filter(|(_, s, _)| s.ready).collect::<Vec<_>>();
	active.sort_by_key(|(e, _, _)| *e);
	let mut herd = herds.iter().filter(|(_, s)| s.ready && s.has_optical()).collect::<Vec<_>>();
	herd.sort_by_key(|(e, _)| *e);
	let mut all = choreography.iter().filter(|(_, c, o)| c.ready && o.ready).collect::<Vec<_>>();
	all.sort_by_key(|(e, _, _)| *e);
	let mut region_ids = Vec::new();
	cache_input.1.clear();
	for (entity, c, o) in &all {
		let Some(region) = &c.region else {
			continue;
		};
		region_ids.push((*entity, 0));
		let Some(asset) = assets.get(&o.part.assets) else {
			continue;
		};
		let Some(v) = asset
			.bodies
			.iter()
			.chain(asset.clouds.iter())
			.map(|h| volumes.get(h))
			.collect::<Option<Vec<_>>>()
		else {
			continue;
		};
		let Ok(v): Result<[&StaticVolume; 7], _> = v.try_into() else {
			continue;
		};
		let revision = region.inputs.as_ref().map_or(region.base.version, |i| i.revision);
		let unchanged = renderer.regions.get(&(*entity, 0)).is_some_and(|r| {
			r.tick == c.tick && r.revision == revision && r.options == o.part.options
		});
		if !unchanged {
			let placement = region.frame.world_from_local;
			let mut primitives = o.primitives(
				c,
				std::array::from_fn(|i| {
					(
						Vec3::from_array(asset.manifest.rest_support[i].min),
						Vec3::from_array(asset.manifest.rest_support[i].max),
					)
				}),
			);
			for p in &mut primitives {
				*p = p.placed(placement);
			}
			let mut payload = renderer
				.regions
				.get_mut(&(*entity, 0))
				.map(|r| std::mem::take(&mut r.payload))
				.unwrap_or_default();
			payload.build(&primitives, v, &asset.style, o.part.options.quality);
			payload.colliders(&region.world_colliders(c.tick as f64 / 120.));
			if let Some(r) = renderer.regions.get_mut(&(*entity, 0)) {
				if let Some(mut b) = buffers.get_mut(&r.scene.buffer) {
					b.clear();
					b.extend_from_slice(&payload.values);
					b.resize_buffer(REGION_PAYLOAD_BYTES);
				}
				r.payload = payload;
				r.tick = c.tick;
				r.revision = revision;
				r.options = o.part.options;
				r.generation = r.generation.wrapping_add(1);
			} else {
				let mut b = ShaderBuffer::from(payload.values.clone());
				b.resize_buffer(REGION_PAYLOAD_BYTES);
				let buffer = buffers.add(b);
				let field = images.add(crate::parts::combined::cache::image());
				let material = materials.add(CombinedMaterial {
					shoulder: v[0].image.clone(),
					column: v[1].image.clone(),
					swept: v[2].image.clone(),
					billow: v[3].image.clone(),
					curl: v[4].image.clone(),
					trail: v[5].image.clone(),
					cluster: v[6].image.clone(),
					payload: buffer.clone(),
					field: field.clone(),
				});
				let quad = meshes.add(Rectangle::new(1., 1.));
				let e = commands
					.spawn((
						Name::new("Contact region optical composition"),
						Mesh3d(quad),
						MeshMaterial3d(material.clone()),
						Transform::from_translation(
							placement.transform_point3(Vec3::new(1., 1., 0.)),
						),
						NoFrustumCulling,
					))
					.id();
				renderer.regions.insert(
					(*entity, 0),
					RegionScene {
						scene: Scene { entity: e, _material: material, buffer, field },
						payload,
						tick: c.tick,
						revision,
						options: o.part.options,
						generation: 1,
						capacity: REGION_PAYLOAD_BYTES,
					},
				);
			}
		}
		if let Some(r) = renderer.regions.get(&(*entity, 0)) {
			cache_input.1.push(crate::parts::combined::cache::Input {
				textures: std::array::from_fn(|i| v[i].image.clone()),
				field: r.scene.field.clone(),
				payload: r.scene.buffer.clone(),
				generation: r.generation,
				dimensions: if o.part.options.quality == CombinedQuality::Low {
					crate::parts::combined::cache::LOW_SIZE
				} else {
					crate::parts::combined::cache::MAX_SIZE
				},
			});
		}
	}
	for (entity, state) in &herd {
		if !state.part.settings.travelling {
			continue;
		}
		for tile in 0..4 {
			if state.tiles[tile].is_none() {
				continue;
			}
			let options = crate::composition::ChoreographyOptions::default();
			region_ids.push((*entity, tile as u8 + 1));
			let Some(asset) = assets.get(&state.part.assets) else {
				continue;
			};
			let Some(v) = asset
				.bodies
				.iter()
				.chain(asset.clouds.iter())
				.map(|h| volumes.get(h))
				.collect::<Option<Vec<_>>>()
			else {
				continue;
			};
			let Ok(v): Result<[&StaticVolume; 7], _> = v.try_into() else {
				continue;
			};
			let revision = state.terrain_version();
			let unchanged = renderer.regions.get(&(*entity, tile as u8 + 1)).is_some_and(|r| {
				r.tick == state.tick && r.revision == revision && r.options == options
			});
			if !unchanged {
				let primitives = state.optical_tile(
					tile,
					std::array::from_fn(|i| {
						(
							Vec3::from_array(asset.manifest.rest_support[i].min),
							Vec3::from_array(asset.manifest.rest_support[i].max),
						)
					}),
				);
				let center = primitives.first().map_or(Vec3::ZERO, |p| (p.min + p.max) * 0.5);

				let mut payload = renderer
					.regions
					.get_mut(&(*entity, tile as u8 + 1))
					.map(|r| std::mem::take(&mut r.payload))
					.unwrap_or_default();
				payload.build(&primitives, v, &asset.style, options.quality);
				payload.colliders(&state.optical_colliders());
				payload.terrain(state.optical_terrain(tile));
				if let Some(r) = renderer.regions.get_mut(&(*entity, tile as u8 + 1)) {
					if let Some(mut b) = buffers.get_mut(&r.scene.buffer) {
						b.clear();
						b.extend_from_slice(&payload.values);
						b.resize_buffer(HERD_PAYLOAD_BYTES);
					}
					r.payload = payload;
					r.tick = state.tick;
					r.revision = revision;
					r.options = options;
					r.generation = r.generation.wrapping_add(1);
				} else {
					let mut b = ShaderBuffer::from(payload.values.clone());
					b.resize_buffer(HERD_PAYLOAD_BYTES);
					let buffer = buffers.add(b);
					let field = images.add(crate::parts::combined::cache::image());
					let material = materials.add(CombinedMaterial {
						shoulder: v[0].image.clone(),
						column: v[1].image.clone(),
						swept: v[2].image.clone(),
						billow: v[3].image.clone(),
						curl: v[4].image.clone(),
						trail: v[5].image.clone(),
						cluster: v[6].image.clone(),
						payload: buffer.clone(),
						field: field.clone(),
					});
					let quad = meshes.add(Rectangle::new(1., 1.));
					let e = commands
						.spawn((
							Name::new("Moving herd optical tile"),
							Mesh3d(quad),
							MeshMaterial3d(material.clone()),
							Transform::from_translation(center),
							NoFrustumCulling,
						))
						.id();
					renderer.regions.insert(
						(*entity, tile as u8 + 1),
						RegionScene {
							scene: Scene { entity: e, _material: material, buffer, field },
							payload,
							tick: state.tick,
							revision,
							options: options,
							generation: 1,
							capacity: HERD_PAYLOAD_BYTES,
						},
					);
				}
			}
			if let Some(r) = renderer.regions.get(&(*entity, tile as u8 + 1)) {
				cache_input.1.push(crate::parts::combined::cache::Input {
					textures: std::array::from_fn(|i| v[i].image.clone()),
					field: r.scene.field.clone(),
					payload: r.scene.buffer.clone(),
					generation: r.generation,
					dimensions: if options.quality == CombinedQuality::Low {
						crate::parts::combined::cache::LOW_SIZE
					} else {
						crate::parts::combined::cache::MAX_SIZE
					},
				});
			}
		}
	}

	let expired = renderer
		.regions
		.keys()
		.filter(|e| !region_ids.contains(e))
		.copied()
		.collect::<Vec<_>>();
	for e in expired {
		if let Some(r) = renderer.regions.remove(&e) {
			commands.entity(r.scene.entity).try_despawn();
		}
	}
	herd.retain(|(_, s)| !s.part.settings.travelling);
	let mut choreography =
		all.into_iter().filter(|(_, c, _)| c.region.is_none()).collect::<Vec<_>>();
	choreography.sort_by_key(|(e, _, _)| *e);
	if active.is_empty() && herd.is_empty() && choreography.is_empty() {
		if let Some(scene) = renderer.scene.take() {
			commands.entity(scene.entity).try_despawn();
		}
		cache_input.0 = None;
		renderer.signature.clear();
		renderer.primitives = 0;
		renderer.grain_count = 0;
		renderer.payload.values.clear();
		renderer.payload.references = 0;
		renderer.payload.max_bin = 0;
		return;
	}
	let mut signature = active
		.iter()
		.map(|(e, s, g)| {
			(
				*e,
				s.simulation.tick,
				s.body_time.to_bits(),
				(s.options.motion as u32) << 24
					| (s.options.quality as u32) << 16
					| (s.options.scenario as u32) << 8
					| (s.options.layers as u32),
				g.to_matrix(),
			)
		})
		.collect::<Vec<_>>();
	signature.extend(herd.iter().map(|(e, s)| (*e, s.tick, 0, 0, Mat4::IDENTITY)));
	signature.extend(choreography.iter().map(|(e, c, o)| {
		(
			*e,
			c.tick,
			o.part.options.body_concentration.to_bits(),
			o.part.options.fine_concentration.to_bits()
				^ ((o.part.options.layers as u32) << 24)
				^ ((o.part.options.quality as u32) << 16),
			Mat4::IDENTITY,
		)
	}));
	let interactive = active.iter().find(|(_, s, _)| s.options.motion.interactive());
	let version = if interactive.is_some() || choreography.iter().any(|(_, _, o)| o.contact()) {
		interaction.as_ref().map_or(0, |i| i.version)
	} else {
		0
	};
	if renderer.scene.is_some()
		&& renderer.signature == signature
		&& renderer.scene_version == version
	{
		return;
	}
	renderer.signature = signature;
	renderer.scene_version = version;
	renderer.generation = renderer.generation.wrapping_add(1);
	let asset_handle = active
		.first()
		.map(|(_, s, _)| &s.assets)
		.or_else(|| herd.first().map(|(_, s)| &s.part.assets))
		.or_else(|| choreography.first().map(|(_, _, o)| &o.part.assets));
	let Some(asset) = asset_handle.and_then(|h| assets.get(h)) else {
		return;
	};
	let handles = asset.bodies.iter().chain(asset.clouds.iter()).collect::<Vec<_>>();
	let Some(v) = handles.iter().map(|h| volumes.get(*h)).collect::<Option<Vec<_>>>() else {
		return;
	};
	let Ok(v): Result<[&StaticVolume; 7], _> = v.try_into() else {
		return;
	};
	let mut primitives = Vec::with_capacity(MAX_PRIMITIVES);
	let mut quality = if herd.is_empty() { CombinedQuality::Low } else { CombinedQuality::Default };
	for (_, state, global) in &active {
		if state.options.quality == CombinedQuality::Default {
			quality = CombinedQuality::Default;
		}
		let placement =
			if state.options.motion.interactive() { Mat4::IDENTITY } else { global.to_matrix() };
		let enabled = |role| state.enabled(role);
		if enabled(crate::composition::CombinedRole::Body) {
			for id in 0..15 {
				let Some(pose) = state.simulation.pose(state.body_time as f64, id) else {
					continue;
				};
				let support = &asset.manifest.rest_support[pose.variant];
				let (lo, hi) =
					pose.bounds(Vec3::from_array(support.min), Vec3::from_array(support.max));
				let (min, max) = Primitive::transformed_bounds(placement, lo, hi);
				let puff = state.simulation.interactive.as_ref().and_then(|s| s.puffs[id].as_ref());
				let model =
					placement * puff.map_or_else(|| pose.transform().to_matrix(), |p| p.model());
				let (min, max) = if let Some(p) = puff {
					let c = p.position.as_vec3();
					let environmental = Mat4::from_translation(c)
						* Mat4::from_mat3(p.affine)
						* Mat4::from_translation(-c);
					Primitive::transformed_bounds(environmental, min, max)
				} else {
					(min, max)
				};
				let seed = crate::parts::combined::simulation::mix32(
					13 ^ ((id as u32 + 1).wrapping_mul(0x27d4eb2d)),
				);
				primitives.push(Primitive {
					inverse: model.inverse(),
					masked: state.options.motion.interactive(),
					coefficients: pose.coefficients.0,
					parameters: Vec4::new(
						pose.variant as f32,
						(seed & 0xffff) as f32 * 0.000219,
						state
							.simulation
							.interactive
							.as_ref()
							.and_then(|s| s.puffs[id].as_ref())
							.map_or(1., |p| p.concentration),
						0.,
					),
					min,
					max,
				});
			}
		}
		for p in &state.simulation.particles {
			if p.age >= p.life {
				continue;
			}
			if p.kind == 0 && !enabled(crate::composition::CombinedRole::Clouds)
				|| p.kind == 1 && !enabled(crate::composition::CombinedRole::Grains)
			{
				continue;
			}
			if p.kind == 0 && p.concentration <= 0. {
				continue;
			}
			let model = Mat4::from_scale_rotation_translation(
				p.scale.as_vec3(),
				p.rotation.as_quat(),
				p.position.as_vec3(),
			);
			let (min, max) = Primitive::transformed_bounds(model, -Vec3::ONE, Vec3::ONE);
			let parameters = if p.kind == 0 {
				Vec4::new((p.variant + 3) as f32, 0., p.concentration as f32, 1.)
			} else {
				p.color.as_vec3().extend(2.)
			};
			primitives.push(Primitive {
				inverse: model.inverse(),
				masked: state.options.motion.interactive(),
				coefficients: Vec4::ZERO,
				parameters,
				min,
				max,
			});
		}
	}
	for (_, s) in &herd {
		primitives.extend(s.optical_primitives(std::array::from_fn(|i| {
			(Vec3::from_array(v[i].manifest.bounds.min), Vec3::from_array(v[i].manifest.bounds.max))
		})));
	}
	for (_, c, o) in &choreography {
		if o.part.options.quality == CombinedQuality::Default {
			quality = CombinedQuality::Default;
		}
		primitives.extend(o.primitives(
			c,
			std::array::from_fn(|i| {
				(
					Vec3::from_array(asset.manifest.rest_support[i].min),
					Vec3::from_array(asset.manifest.rest_support[i].max),
				)
			}),
		));
	}
	if primitives.len() > MAX_PRIMITIVES {
		warn!("Combined shared primitive capacity exceeded");
		return;
	}
	renderer.grain_count = primitives.iter().filter(|p| p.parameters.w == 2.).count();
	renderer.primitives = primitives.len();
	let style = asset.style.clone();
	renderer.payload.build(&primitives, v, &style, quality);
	let mut colliders = interactive
		.and_then(|(_, s, _)| {
			interaction.as_ref().map(|i| {
				i.colliders(s.simulation.time())
					.into_iter()
					.take(i.collider_count())
					.collect::<Vec<_>>()
			})
		})
		.unwrap_or_default();
	for (_, c, o) in &choreography {
		if o.contact() {
			if let Some(i) = interaction.as_deref() {
				colliders
					.extend(i.colliders(c.tick as f64 / 120.).into_iter().take(i.collider_count()));
			}
		}
	}
	for (_, s) in &herd {
		colliders.extend(s.optical_colliders());
	}
	if colliders.len() > 28 {
		warn!("Combined shared collider capacity exceeded");
		return;
	}
	renderer.payload.colliders(&colliders);
	if renderer.scene.is_none() {
		let mut buffer = ShaderBuffer::from(renderer.payload.values.clone());
		buffer.resize_buffer(MAX_PAYLOAD_BYTES);
		buffer.label = "Combined dust bounded scene".into();
		let buffer = buffers.add(buffer);
		let field = images.add(crate::parts::combined::cache::image());
		let material = materials.add(CombinedMaterial {
			shoulder: v[0].image.clone(),
			column: v[1].image.clone(),
			swept: v[2].image.clone(),
			billow: v[3].image.clone(),
			curl: v[4].image.clone(),
			trail: v[5].image.clone(),
			cluster: v[6].image.clone(),
			payload: buffer.clone(),
			field: field.clone(),
		});
		// A single screen quad, not one surface shell per volume. Fragment reconstructs world rays.
		let mesh = meshes.add(Rectangle::new(1., 1.));
		let entity = commands
			.spawn((
				Name::new("Combined shared optical composition"),
				Mesh3d(mesh),
				MeshMaterial3d(material.clone()),
				Transform::IDENTITY,
				NoFrustumCulling,
			))
			.id();
		renderer.scene = Some(Scene { entity, _material: material, buffer, field });
	} else if let Some(scene) = &renderer.scene {
		if let Some(mut buffer) = buffers.get_mut(&scene.buffer) {
			buffer.clear();
			buffer.extend_from_slice(&renderer.payload.values);
			buffer.resize_buffer(MAX_PAYLOAD_BYTES);
		}
	}
	if let Some(scene) = &renderer.scene {
		cache_input.0 = Some(crate::parts::combined::cache::Input {
			textures: std::array::from_fn(|i| v[i].image.clone()),
			field: scene.field.clone(),
			payload: scene.buffer.clone(),
			generation: renderer.generation,
			dimensions: if quality == CombinedQuality::Low {
				crate::parts::combined::cache::LOW_SIZE
			} else {
				crate::parts::combined::cache::MAX_SIZE
			},
		});
	}
}

#[cfg(test)]
mod herd_capacity_tests {
	use crate::parts::combined::{
		interaction::InteractionScene,
		render::{Payload, MAX_PAYLOAD_BYTES, MAX_PRIMITIVES},
	};
	use bevy::prelude::*;
	#[test]
	fn uploads_twenty_colliders_and_reserves_all_herd_primitives() {
		let scene = InteractionScene::default();
		let c = scene.colliders(0.)[0];
		let colliders = vec![c; 20];
		let mut p = Payload::default();
		p.values.resize(22, Vec4::ZERO);
		p.colliders(&colliders);
		assert_eq!(p.values[21].x, 20.);
		assert_eq!(p.values.len(), 22 + 80);
		assert!(MAX_PRIMITIVES >= 4 * (15 + 96 + 96) + 32 + 64 + 64);
		assert!(MAX_PAYLOAD_BYTES < 128 * 1024 * 1024);
	}
}

#[cfg(test)]
mod placement_tests {
	use super::*;
	#[test]
	fn density_inverse_bounds_and_normals_transform_once() {
		let placement = Mat4::from_rotation_translation(
			Quat::from_rotation_y(0.64577),
			Vec3::new(-12., 8., 18.),
		);
		let model = Mat4::from_scale_rotation_translation(
			Vec3::new(0.6, 0.8, 0.7),
			Quat::from_rotation_y(0.3),
			Vec3::new(1., 0.2, -0.4),
		);
		let (min, max) = Primitive::transformed_bounds(model, Vec3::splat(-1.), Vec3::splat(1.));
		let p = Primitive {
			inverse: model.inverse(),
			masked: true,
			coefficients: Vec4::ZERO,
			parameters: Vec4::new(1., 0.3, 1., 0.),
			min,
			max,
		};
		let q = p.placed(placement);
		for x in [-1., 0., 1.] {
			for y in [-1., 0.3, 1.] {
				for z in [-1., 0.6, 1.] {
					let rest = Vec3::new(x, y, z);
					let world = (placement * model).transform_point3(rest);
					assert!(q.inverse.transform_point3(world).abs_diff_eq(rest, 1e-5));
					assert!(
						world.cmpge(q.min - Vec3::splat(1e-5)).all()
							&& world.cmple(q.max + Vec3::splat(1e-5)).all()
					);
				}
			}
		}
		let normal = Vec3::new(0.4, 0.8, 0.1).normalize();
		let expected =
			(placement * model).inverse().transpose().transform_vector3(normal).normalize();
		let via_placement = placement
			.transform_vector3(model.inverse().transpose().transform_vector3(normal))
			.normalize();
		assert!(expected.abs_diff_eq(via_placement, 1e-6));
	}
}
