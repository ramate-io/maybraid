//! Bounded density integration with two time samples, shared 3D images, and instance clocks.
use crate::membership::VfxMemberOf;
use crate::spawn::{VfxInstance, VfxLayerLife, VfxSpawn};
use crate::{
	PackagedVolumeFlipbook, VfxLayerInfo, VfxLoadFailure, VolumeFlipbookPart, VolumeFlipbookRef,
};
use bevy::asset::{embedded_asset, LoadState, RecursiveDependencyLoadState};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
	AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

impl VolumeFlipbookPart {
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) -> Entity {
		commands
			.spawn((
				Name::new("vfx-layer-volume-flipbook"),
				ChildOf(parent),
				VfxMemberOf(parent),
				transform,
				Visibility::Hidden,
				NoFrustumCulling,
				NotShadowCaster,
				VfxVolumeFlipbook {
					flipbook: self.flipbook.clone(),
					tint: {
						let a = self.tint.to_linear();
						let b = spawn.tint_or_white().to_linear();
						Color::linear_rgba(
							a.red * b.red,
							a.green * b.green,
							a.blue * b.blue,
							a.alpha * b.alpha,
						)
					},
					illumination: self.illumination,
					intensity: spawn.clamped_intensity(),
					started: false,
				},
				VfxLayerLife {
					age: 0.,
					duration: 0.,
					playback: spawn.clamped_playback(),
					waiting_for_emitter: true,
				},
			))
			.id()
	}
}

#[derive(Component)]
pub struct VfxVolumeFlipbook {
	pub illumination: Vec4,
	pub flipbook: VolumeFlipbookRef,
	pub tint: Color,
	pub intensity: f32,
	started: bool,
}

#[derive(Component)]
pub(crate) struct ResolvedVolume(pub Handle<PackagedVolumeFlipbook>);
#[derive(Resource, Default)]
pub(crate) struct VolumeProxyMesh(Option<Handle<Mesh>>);

pub(crate) fn resolve_volumes(
	mut commands: Commands,
	server: Res<AssetServer>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut proxy: ResMut<VolumeProxyMesh>,
	pending: Query<(Entity, &VfxVolumeFlipbook), Without<ResolvedVolume>>,
) {
	for (entity, part) in &pending {
		let mesh = proxy.0.get_or_insert_with(|| meshes.add(Rectangle::default()));
		commands
			.entity(entity)
			.try_insert((ResolvedVolume(part.flipbook.load(&server)), Mesh3d(mesh.clone())));
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct VolumeFlipbookMaterial {
	#[uniform(9)]
	pub illumination: Vec4,
	#[texture(0, dimension = "3d")]
	#[sampler(1)]
	pub frame_a: Handle<Image>,
	#[texture(2, dimension = "3d")]
	pub frame_b: Handle<Image>,
	#[uniform(3)]
	pub local_to_texture: Mat4,
	#[uniform(4)]
	pub world_to_local: Mat4,
	#[uniform(5)]
	pub bounds_min: Vec4,
	#[uniform(6)]
	pub bounds_max: Vec4,
	/// Interpolation, fade opacity, density scale, transform validity.
	#[uniform(7)]
	pub settings: Vec4,
	#[uniform(8)]
	pub tint: Vec4,
}

impl VolumeFlipbookMaterial {
	/// Construct from a loader-validated asset (at least two frame handles).
	pub fn new(asset: &PackagedVolumeFlipbook, tint: Color, intensity: f32) -> Self {
		let tint = LinearRgba::from(tint);
		Self {
			illumination: Vec4::ZERO,
			frame_a: asset.frames[0].clone(),
			frame_b: asset.frames[1].clone(),
			local_to_texture: asset.manifest.local_to_texture(),
			world_to_local: Mat4::IDENTITY,
			bounds_min: Vec3::from_array(asset.manifest.bounds.min).extend(0.),
			bounds_max: Vec3::from_array(asset.manifest.bounds.max).extend(0.),
			settings: Vec4::new(0., 1., asset.manifest.density_scale * intensity, 1.),
			tint: Vec4::new(tint.red, tint.green, tint.blue, tint.alpha),
		}
	}

	pub fn set_age(&mut self, asset: &PackagedVolumeFlipbook, age: f32) {
		let clock = asset.manifest.clock_at(age);
		self.frame_a = asset.frames[clock.frame_0 as usize].clone();
		self.frame_b = asset.frames[clock.frame_1 as usize].clone();
		self.settings.x = clock.fract;
		self.settings.y = clock.opacity;
	}

	pub fn set_transform(&mut self, transform: &GlobalTransform) {
		let matrix = transform.to_matrix();
		let inverse = matrix.inverse();
		self.settings.w = if inverse.is_finite() { 1. } else { 0. };
		self.world_to_local = if inverse.is_finite() { inverse } else { Mat4::IDENTITY };
	}
}

impl Material for VolumeFlipbookMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/parts/volume_flipbook/volume_flipbook.wesl"
		)
		.into()
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
		descriptor: &mut RenderPipelineDescriptor,
		_: &MeshVertexBufferLayoutRef,
		_: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			// The proxy covers projected bounds, not an actual surface. Clip the ray against prepass depth instead.
			depth.depth_write_enabled = Some(false);
			depth.depth_compare = Some(CompareFunction::Always);
		}
		Ok(())
	}
}

pub struct VolumeFlipbookPlugin;
impl Plugin for VolumeFlipbookPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "volume_flipbook/volume_flipbook.wesl");
		app.init_resource::<VolumeProxyMesh>()
			.add_message::<VfxLoadFailure>()
			.add_plugins(MaterialPlugin::<VolumeFlipbookMaterial>::default())
			.add_systems(
				PostUpdate,
				sync_volume_placement.after(bevy::transform::TransformSystems::Propagate),
			);
	}
}

pub(crate) fn sync_volume_placement(
	layers: Query<(&GlobalTransform, &MeshMaterial3d<VolumeFlipbookMaterial>)>,
	mut materials: ResMut<Assets<VolumeFlipbookMaterial>>,
) {
	for (transform, handle) in &layers {
		if let Some(mut material) = materials.get_mut(&handle.0) {
			material.set_transform(transform);
		}
	}
}

pub(crate) fn tick_volumes(
	mut commands: Commands,
	time: Res<Time>,
	server: Res<AssetServer>,
	instances: Query<&VfxInstance>,
	assets: Res<Assets<PackagedVolumeFlipbook>>,
	images: Res<Assets<Image>>,
	mut materials: ResMut<Assets<VolumeFlipbookMaterial>>,
	mut failures: MessageWriter<VfxLoadFailure>,
	mut layers: Query<(
		Entity,
		&mut VfxVolumeFlipbook,
		&ResolvedVolume,
		&VfxMemberOf,
		Option<&VfxLayerInfo>,
		&mut VfxLayerLife,
		&mut Visibility,
		Option<&MeshMaterial3d<VolumeFlipbookMaterial>>,
	)>,
) {
	for (entity, mut part, resolved, member, info, mut life, mut visibility, material) in
		&mut layers
	{
		let error = match server.get_load_states(resolved.0.id()) {
			Some((LoadState::Failed(error), _, _)) => Some(error.to_string()),
			Some((_, _, RecursiveDependencyLoadState::Failed(error))) => Some(error.to_string()),
			_ => None,
		};
		if let Some(error) = error {
			let failure = VfxLoadFailure {
				root: member.root(),
				effect: instances.get(member.root()).map(|v| v.name.clone()).unwrap_or_default(),
				layer: info
					.map(|i| format!("{} ({})", i.index, i.label))
					.unwrap_or_else(|| "volume".into()),
				path: part.flipbook.path.clone(),
				error,
			};
			warn!("{failure}");
			failures.write(failure);
			commands.entity(member.root()).try_despawn();
			continue;
		}
		let Some(asset) = assets.get(&resolved.0) else {
			continue;
		};
		if !server.is_loaded_with_dependencies(resolved.0.id())
			|| asset.frames.iter().any(|frame| !images.contains(frame))
		{
			continue;
		}
		let Some(handle) = material else {
			commands.entity(entity).try_insert(MeshMaterial3d(materials.add({
				let mut m = VolumeFlipbookMaterial::new(asset, part.tint, part.intensity);
				m.illumination = part.illumination;
				m
			})));
			continue;
		};
		life.duration = asset.manifest.duration_seconds();
		life.waiting_for_emitter = false;
		if !member.instance_armed(&instances) {
			continue;
		}
		if part.started {
			life.age += time.delta_secs() * life.playback;
		}
		part.started = true;
		*visibility = Visibility::Inherited;
		if let Some(mut material) = materials.get_mut(&handle.0) {
			material.set_age(asset, life.age);
		}
	}
}

#[cfg(test)]
mod tests;
