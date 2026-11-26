//! Capture-plane renderer for packaged KTX flipbooks. Shared texture, per-instance clock.

use bevy::asset::{embedded_asset, LoadState, RecursiveDependencyLoadState};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use crate::composition::FlipbookPart;
use crate::flipbook_ref::FlipbookRef;
use crate::inspection::{VfxLayerInfo, VfxLoadFailure};
use crate::membership::VfxMemberOf;
use crate::parts::packaged::{AlphaConvention, PackagedFlipbook};
use crate::spawn::{VfxInstance, VfxLayerLife, VfxSpawn};

impl FlipbookPart {
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) -> Entity {
		commands
			.spawn((
				Name::new("vfx-layer-flipbook"),
				ChildOf(parent),
				VfxMemberOf(parent),
				transform,
				Visibility::Hidden,
				NoFrustumCulling,
				NotShadowCaster,
				VfxDust {
					flipbook: self.flipbook.clone(),
					tint: spawn.tint_or_white(),
					intensity: spawn.clamped_intensity(),
				},
				VfxLayerLife {
					age: 0.0,
					duration: 0.0,
					playback: spawn.clamped_playback(),
					waiting_for_emitter: true,
				},
			))
			.id()
	}
}

#[derive(Component)]
pub struct VfxDust {
	pub flipbook: FlipbookRef,
	pub tint: Color,
	pub intensity: f32,
}

/// Resolved handle belongs to the runtime child, never the recipe or pending layer.
#[derive(Component)]
pub(crate) struct ResolvedFlipbook(pub Handle<PackagedFlipbook>);

/// One standard proxy for all flipbook styles and instances in this world.
#[derive(Resource, Default)]
pub(crate) struct FlipbookProxyMesh(Option<Handle<Mesh>>);

pub(crate) fn resolve_flipbooks(
	mut commands: Commands,
	server: Res<AssetServer>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut proxy: ResMut<FlipbookProxyMesh>,
	pending: Query<(Entity, &VfxDust), Without<ResolvedFlipbook>>,
) {
	for (entity, dust) in &pending {
		let mesh = proxy.0.get_or_insert_with(|| meshes.add(Rectangle::default()));
		commands
			.entity(entity)
			.try_insert((ResolvedFlipbook(dust.flipbook.load(&server)), Mesh3d(mesh.clone())));
	}
}

/// One material per effect layer, never per sprite. The texture remains shared.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DustMaterial {
	#[texture(0, dimension = "2d_array")]
	#[sampler(1)]
	pub image: Handle<Image>,
	/// frame 0, frame 1, interpolation, final opacity.
	#[uniform(2)]
	pub time: Vec4,
	/// atlas width/height, grid width, frame count.
	#[uniform(3)]
	pub atlas: Vec4,
	/// frame interior width/height, gutter, straight alpha flag.
	#[uniform(4)]
	pub frame: Vec4,
	/// local quad center relative to pivot, first capture azimuth radians.
	#[uniform(5)]
	pub center: Vec4,
	/// capture width/height, elevation radians, view count.
	#[uniform(6)]
	pub capture: Vec4,
	#[uniform(7)]
	pub tint: Vec4,
}

impl DustMaterial {
	/// This renderer blends a single evenly spaced ring of authored capture planes.
	pub fn supports(asset: &PackagedFlipbook) -> bool {
		asset.manifest.azimuth_ring().is_some()
	}

	pub fn new(asset: &PackagedFlipbook, tint: Color, intensity: f32) -> Self {
		let m = &asset.manifest;
		let center = Vec3::from_array(m.quad_center) - Vec3::from_array(m.pivot);
		let first = &m.views[0]; // Validated nonempty by the packaged loader.
		let tint = LinearRgba::from(tint);
		Self {
			image: asset.image.clone(),
			time: Vec4::new(0.0, 1.0, 0.0, 1.0),
			atlas: Vec4::new(
				m.atlas_size[0] as f32,
				m.atlas_size[1] as f32,
				m.grid[0] as f32,
				m.frame_count as f32,
			),
			frame: Vec4::new(
				m.frame_size[0] as f32,
				m.frame_size[1] as f32,
				m.gutter_px as f32,
				if m.alpha_mode == AlphaConvention::Straight { 1.0 } else { 0.0 },
			),
			center: center.extend(first.azimuth_degrees.to_radians()),
			capture: Vec4::new(
				m.capture_size[0],
				m.capture_size[1],
				first.elevation_degrees.to_radians(),
				m.views.len() as f32,
			),
			tint: Vec4::new(
				tint.red * intensity,
				tint.green * intensity,
				tint.blue * intensity,
				tint.alpha,
			),
		}
	}

	pub fn set_age(&mut self, asset: &PackagedFlipbook, age: f32) {
		self.time = asset.manifest.clock_at(age).time();
	}
}

impl Material for DustMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/parts/flipbook_card/flipbook_card.wesl")
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
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

pub struct DustPlugin;
impl Plugin for DustPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "flipbook_card/flipbook_card.wesl");
		app.add_message::<VfxLoadFailure>()
			.init_resource::<FlipbookProxyMesh>()
			.add_plugins(MaterialPlugin::<DustMaterial>::default());
	}
}

/// Wait for the complete image dependency before starting this layer's clock.
/// Handles are dropped with the child on cancellation or normal root cleanup.
pub(crate) fn tick_dust(
	mut commands: Commands,
	time: Res<Time>,
	instances: Query<&VfxInstance>,
	assets: Res<Assets<PackagedFlipbook>>,
	images: Res<Assets<Image>>,
	server: Res<AssetServer>,
	mut materials: ResMut<Assets<DustMaterial>>,
	mut failures: MessageWriter<VfxLoadFailure>,
	mut dust: Query<(
		Entity,
		&VfxDust,
		&ResolvedFlipbook,
		&VfxMemberOf,
		Option<&VfxLayerInfo>,
		&mut VfxLayerLife,
		&mut Visibility,
		Option<&MeshMaterial3d<DustMaterial>>,
	)>,
) {
	for (entity, dust, resolved, member, info, mut life, mut visibility, material) in &mut dust {
		let report = |error: String| VfxLoadFailure {
			root: member.root(),
			effect: instances.get(member.root()).map(|v| v.name.clone()).unwrap_or_default(),
			layer: info
				.map(|v| format!("{} ({})", v.index, v.label))
				.unwrap_or_else(|| "flipbook".into()),
			path: dust.flipbook.path.clone(),
			error,
		};
		let error = match server.get_load_states(resolved.0.id()) {
			Some((LoadState::Failed(error), _, _)) => Some(error.to_string()),
			Some((_, _, RecursiveDependencyLoadState::Failed(error))) => Some(error.to_string()),
			_ => None,
		};
		if let Some(error) = error {
			let failure = report(error);
			warn!("{failure}");
			failures.write(failure);
			commands.entity(member.root()).try_despawn();
			continue;
		}
		let Some(asset) = assets.get(&resolved.0) else {
			continue;
		};
		if !DustMaterial::supports(asset) {
			let failure = report("renderer requires evenly spaced views at one elevation".into());
			warn!("{failure}");
			failures.write(failure);
			commands.entity(member.root()).try_despawn();
			continue;
		}
		if !images.contains(&asset.image) {
			continue;
		}
		if server.get_load_states(resolved.0.id()).is_some()
			&& !server.is_loaded_with_dependencies(resolved.0.id())
		{
			continue;
		}
		let Some(handle) = material else {
			commands.entity(entity).insert(MeshMaterial3d(materials.add(DustMaterial::new(
				asset,
				dust.tint,
				dust.intensity,
			))));
			continue;
		};
		life.waiting_for_emitter = false;
		life.duration = asset.manifest.duration_seconds();
		if !member.instance_armed(&instances) {
			continue;
		}
		if *visibility != Visibility::Hidden {
			life.age += time.delta_secs() * life.playback;
		}
		*visibility = Visibility::Inherited;
		if let Some(mut material) = materials.get_mut(&handle.0) {
			material.set_age(asset, life.age);
		}
	}
}

#[cfg(test)]
mod tests;
