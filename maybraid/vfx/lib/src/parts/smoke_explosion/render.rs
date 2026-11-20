//! Bounded per-instance optical resources, shared immutable rest fields.
use crate::{
	composition::{ExplosionDiagnostic, ExplosionQuality},
	parts::{
		packaged::StaticVolume,
		smoke_explosion::{density::CombinedExplosionAsset, track::ExplosionTrack, ExplosionLayer},
	},
};
use bevy::{
	asset::{embedded_asset, RenderAssetUsages},
	camera::visibility::NoFrustumCulling,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	render::{render_resource::*, storage::ShaderBuffer},
	shader::ShaderRef,
};
mod compute;
pub use compute::GpuReadiness;
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ExplosionVolumeMaterial {
	#[texture(0, dimension = "3d")]
	#[sampler(1)]
	pub field: Handle<Image>,
	#[storage(2, read_only)]
	pub payload: Handle<ShaderBuffer>,
}
impl Material for ExplosionVolumeMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/smoke_explosion/volume.wesl")
			.into()
	}
	fn fragment_shader() -> ShaderRef {
		Self::vertex_shader()
	}
	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Premultiplied
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
		if let Some(d) = p.depth_stencil.as_mut() {
			d.depth_write_enabled = Some(false);
			d.depth_compare = Some(CompareFunction::Always);
		}
		Ok(())
	}
}
struct Resources {
	material: Handle<ExplosionVolumeMaterial>,
	payload: Handle<ShaderBuffer>,
	density: Handle<Image>,
	light: Handle<Image>,
	proxy: Entity,
	grain_entities: Vec<Entity>,
	grain_block: usize,
	dimensions: UVec3,
	last: f32,
	generation: u64,
}
#[derive(Resource, Default)]
struct Owned(std::collections::HashMap<Entity, Resources>);
#[derive(Resource)]
struct Shared {
	screen: Handle<Mesh>,
	grain: Handle<Mesh>,
}
fn setup(mut c: Commands, mut meshes: ResMut<Assets<Mesh>>) {
	c.insert_resource(Shared {
		screen: meshes.add(Rectangle::new(1., 1.)),
		grain: meshes.add(grain_mesh()),
	});
}
pub fn plugin(app: &mut App) {
	embedded_asset!(app, "volume.wesl");
	embedded_asset!(app, "compose.wesl");
	embedded_asset!(app, "light.wesl");
	app.add_plugins(MaterialPlugin::<ExplosionVolumeMaterial>::default())
		.init_resource::<Owned>()
		.add_systems(Startup, setup)
		.add_systems(
			PostUpdate,
			update
				.after(bevy::transform::TransformSystems::Propagate)
				.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
		);
	compute::plugin(app);
}
fn update(
	mut c: Commands,
	layers: Query<(Entity, &GlobalTransform, &ExplosionLayer)>,
	tracks: Res<Assets<ExplosionTrack>>,
	assets: Res<Assets<CombinedExplosionAsset>>,
	volumes: Res<Assets<StaticVolume>>,
	shared: Res<Shared>,
	mut owned: ResMut<Owned>,
	mut images: ResMut<Assets<Image>>,
	mut buffers: ResMut<Assets<ShaderBuffer>>,
	mut materials: ResMut<Assets<ExplosionVolumeMaterial>>,
	mut opaque: ResMut<crate::parts::smoke_explosion::cartoon::Pool>,
	mut grain_transforms: Query<
		(&mut Transform, &mut GlobalTransform, &mut Visibility),
		Without<ExplosionLayer>,
	>,
	mut input: ResMut<compute::Inputs>,
	request: Option<Res<DensityReadback>>,
	mut diagnostics: ResMut<crate::parts::smoke_explosion::ExplosionDiagnostics>,
) {
	input.0.clear();
	let dead = owned.0.keys().filter(|e| layers.get(**e).is_err()).copied().collect::<Vec<_>>();
	for id in dead {
		if let Some(r) = owned.0.remove(&id) {
			materials.remove(r.material.id());
			buffers.remove(r.payload.id());
			images.remove(r.density.id());
			images.remove(r.light.id());

			c.entity(r.proxy).try_despawn();
			for e in r.grain_entities {
				c.entity(e).try_despawn();
			}
		}
	}
	for (id, world, layer) in &layers {
		let Some(part) = &layer.combined else {
			continue;
		};
		if !layer.ready || layer.last_time < 0. {
			continue;
		}
		let Some(asset) = assets.get(&part.asset) else {
			continue;
		};
		let Some(track) = tracks.get(&layer.track) else {
			continue;
		};
		let Some(rest) = asset.fields.iter().map(|h| volumes.get(h)).collect::<Option<Vec<_>>>()
		else {
			continue;
		};
		if !owned.0.contains_key(&id) {
			if let Err(e) = asset.validate_fields(track, &volumes) {
				diagnostics.last_error = Some(e.to_string());
				warn!("CombinedExplosion rejected: {e}");
				c.entity(id).try_despawn();
				continue;
			}
		}
		let t = layer.last_time;
		let free_block =
			(0..3).find(|b| !owned.0.values().any(|r| r.grain_block == *b)).unwrap_or(0);
		let r = owned.0.entry(id).or_insert_with(|| {
			let density = images.add(compute::image(TextureFormat::R16Float));
			let light = images.add(compute::image(TextureFormat::Rgba16Float));
			let payload =
				buffers.add(ShaderBuffer::new(vec![Vec4::ZERO; 44], RenderAssetUsages::all()));
			let material = materials
				.add(ExplosionVolumeMaterial { field: light.clone(), payload: payload.clone() });
			let proxy = c
				.spawn((
					Mesh3d(shared.screen.clone()),
					MeshMaterial3d(material.clone()),
					Transform::IDENTITY,
					NoFrustumCulling,
				))
				.id();
			let mut ge = Vec::new();
			for (i, b) in asset.grains.iter().enumerate() {
				let tag = 8 * 349 + free_block * 96 + i;
				let data = crate::parts::smoke_explosion::cartoon::Surface {
					centroid: Vec4::ZERO,
					opening: Vec4::ZERO,
					controls: Vec4::new(0., 0., 1., 2.),
					effect_from_world: Mat4::IDENTITY,
					tint: Vec4::new(0.10 * b.shade, 0.070 * b.shade, 0.055 * b.shade, 1.)
						* layer.tint,
				};
				opaque.values[tag * 8..tag * 8 + 8].copy_from_slice(&data.rows());
				ge.push(
					c.spawn((
						Mesh3d(shared.grain.clone()),
						MeshMaterial3d(opaque.material.clone()),
						bevy::mesh::MeshTag::new(tag as u32),
						Transform::IDENTITY,
						Visibility::Hidden,
						bevy::light::NotShadowCaster,
					))
					.id(),
				);
			}
			if let Some(mut buffer) = buffers.get_mut(&opaque.buffer) {
				buffer.clear();
				buffer.extend_from_slice(&opaque.values);
			}

			Resources {
				material,
				payload,
				density,
				light,
				proxy,
				grain_entities: ge,
				grain_block: free_block,
				dimensions: UVec3::splat(1),
				last: -1.,
				generation: 0,
			}
		});
		if r.last != t {
			let main = asset.main_primitives(track, &layer.frame, t, &volumes);
			let fine = asset.fine_states(track, t);
			let mut primitive_rows = Vec::with_capacity(main.len() + fine.len());
			let mut bounds = Vec::with_capacity(main.len() + fine.len());
			for p in &main {
				primitive_rows.push(p.rows());
				bounds.push((p.min, p.max));
			}
			for p in &fine {
				primitive_rows.push(p.rows());
				bounds.push((p.min, p.max));
			}
			let mut lo = Vec3::splat(f32::INFINITY);
			let mut hi = -lo;
			for (a, b) in &bounds {
				lo = lo.min(*a);
				hi = hi.max(*b);
			}
			if bounds.is_empty() {
				lo = Vec3::ZERO;
				hi = Vec3::ONE;
			}
			lo -= Vec3::splat(0.07);
			hi += Vec3::splat(0.07);
			lo.y = lo.y.min(0.);
			let target: f32 = match part.quality {
				ExplosionQuality::High => 0.035,
				ExplosionQuality::Medium => 0.055,
				ExplosionQuality::Low => 0.085,
			};
			let step = target.max((hi - lo).max_element() / 254.);
			let dims =
				((hi - lo) / step).ceil().as_uvec3().max(UVec3::splat(2)).min(UVec3::splat(256));
			hi = lo + dims.as_vec3() * step;
			r.dimensions = dims;
			let model = world.to_matrix();
			if let Ok((mut transform, mut global, _)) = grain_transforms.get_mut(r.proxy) {
				transform.translation = model.transform_point3((lo + hi) * 0.5);
				*global = GlobalTransform::from(*transform);
			}
			let inv = model.inverse();
			let diagnostic = match part.diagnostic {
				ExplosionDiagnostic::Composed => 0.,
				ExplosionDiagnostic::Main => 1.,
				ExplosionDiagnostic::Fine => 2.,
				ExplosionDiagnostic::Spikes => 3.,
				ExplosionDiagnostic::Section => 4.,
			};
			let mut payload = vec![Vec4::ZERO; 44];
			payload[0] = lo.extend(t);
			payload[1] = hi.extend(diagnostic);
			payload[2] = dims.as_vec3().extend(step);
			payload[3] = Vec4::new(
				primitive_rows.len() as f32,
				(44 + primitive_rows.len() * 10) as f32,
				0.,
				0.,
			);
			for i in 0..3 {
				payload[4 + i] = inv.row(i);
				payload[7 + i] = model.row(i);
			}
			payload[10] = layer.tint;
			payload[11] = Vec4::new(main.len() as f32, fine.len() as f32, 0., 0.);
			for (i, v) in rest.iter().enumerate() {
				payload[12 + i * 2] = Vec3::from_array(v.manifest.bounds.min).extend(0.);
				payload[13 + i * 2] = Vec3::from_array(v.manifest.bounds.max).extend(0.);
			}
			for rows in &primitive_rows {
				payload.extend_from_slice(rows);
			}
			let binbase = payload.len();
			payload.resize(binbase + 4096, Vec4::ZERO);
			let mut references = Vec::<f32>::new();
			for z in 0..16 {
				for y in 0..16 {
					for x in 0..16 {
						let cell = UVec3::new(x, y, z);
						let a = lo + (hi - lo) * cell.as_vec3() / 16.;
						let b = lo + (hi - lo) * (cell.as_vec3() + Vec3::ONE) / 16.;
						let begin = references.len();
						for (i, (min, max)) in bounds.iter().enumerate() {
							if min.cmple(b).all() && max.cmpge(a).all() {
								references.push(i as f32);
							}
						}
						payload[binbase + (x + 16 * (y + 16 * z)) as usize] =
							Vec4::new(begin as f32, references.len() as f32, 0., 0.);
					}
				}
			}
			payload[3].z = payload.len() as f32;
			for words in references.chunks(4) {
				let mut v = Vec4::ZERO;
				for (i, w) in words.iter().enumerate() {
					v[i] = *w;
				}
				payload.push(v);
			}
			if let Some(mut buffer) = buffers.get_mut(&r.payload) {
				buffer.clear();
				buffer.extend_from_slice(&payload);
			}
			r.generation = r.generation.wrapping_add(1);
			r.last = t;
			for ((e, g), b) in r.grain_entities.iter().zip(&layer.grains.states).zip(&asset.grains)
			{
				if let Ok((mut transform, mut global, mut visibility)) =
					grain_transforms.get_mut(*e)
				{
					if part.diagnostic != ExplosionDiagnostic::Composed
						|| t < b.birth || g.radius < 1e-5
					{
						*visibility = Visibility::Hidden;
						continue;
					}
					*visibility = Visibility::Visible;
					let angle = (b.orient_seed & 65535) as f32 * 0.0001;
					let local = Mat4::from_scale_rotation_translation(
						Vec3::new(1., 0.66, 1.23) * g.radius,
						Quat::from_euler(EulerRot::XYZ, angle, angle * 1.7, angle * 0.73),
						g.position,
					);
					*transform = Transform::from_matrix(model * local);
					// These independently owned world-space meshes are updated after hierarchy propagation.
					// Publish this frame to extraction and sorting, rather than waiting one frame.
					*global = GlobalTransform::from(*transform);
				}
			}
		}
		if let Some(request) = request.as_ref() {
			let path = request.0.clone();
			let field = r.density.clone();
			let id = id.to_bits();
			let t = r.last;
			if let Some(buffer) = buffers.get(&r.payload) {
				if let bevy::render::storage::ShaderBufferData::Initialized { data, .. } =
					&buffer.data
				{
					let _ = std::fs::create_dir_all(&path);
					let _ = std::fs::write(
						path.join(format!("{id}-t{t:.4}-payload.bin")),
						data.as_slice(),
					);
				}
			}
			c.spawn(bevy::render::gpu_readback::Readback::texture(field)).observe(
				move |event: On<bevy::render::gpu_readback::ReadbackComplete>, mut c: Commands| {
					let file = path.join(format!("{id}-t{t:.4}-density.r16"));
					if let Err(e) = std::fs::write(file, &event.data) {
						warn!("explosion density readback: {e}");
					}
					c.entity(event.entity).try_despawn();
				},
			);
			c.remove_resource::<DensityReadback>();
		}
		input.0.push(compute::Input {
			id,
			textures: std::array::from_fn(|i| rest[i].image.clone()),
			density: r.density.clone(),
			light: r.light.clone(),
			payload: r.payload.clone(),
			generation: r.generation,
			dimensions: r.dimensions,
		});
	}
}

/// Diagnostic only: read the frozen composed density and write its exact input payload.
/// The caller must keep readbacks out of timing flights.
#[derive(Resource)]
pub struct DensityReadback(pub std::path::PathBuf);

fn grain_mesh() -> Mesh {
	use bevy::mesh::{Indices, PrimitiveTopology};
	let p = vec![
		[1.02, 0., 0.],
		[0., 1., 0.],
		[0., 0., 0.94],
		[-0.92, 0., 0.],
		[0., 0., -1.06],
		[0., -0.97, 0.],
	];
	let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
	m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
	m.insert_indices(Indices::U32(vec![
		0, 1, 2, 2, 1, 3, 3, 1, 4, 4, 1, 0, 2, 5, 0, 3, 5, 2, 4, 5, 3, 0, 5, 4,
	]));
	m.duplicate_vertices();
	m.compute_flat_normals();
	m
}
