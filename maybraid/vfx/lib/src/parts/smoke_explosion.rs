//! Authored smoke explosions. Shared track, explicitly independent rendering styles.
mod sha256;
#[cfg(test)]
mod tests;
pub mod track;

mod cartoon;
pub mod density;
mod grains;
pub mod render;
use crate::{
	composition::{CartoonExplosionPart, CombinedExplosionPart, ExplosionLod},
	membership::VfxMemberOf,
	spawn::{VfxInstance, VfxLayerLife, VfxSpawn},
};
use bevy::{light::NotShadowCaster, prelude::*};
use cartoon::Surface;
use track::{ExplosionTrack, Frame};
#[derive(Component)]
pub struct ExplosionLayer {
	pub track: Handle<ExplosionTrack>,
	pub combined: Option<CombinedExplosionPart>,
	pub lod: ExplosionLod,
	pub frame: Frame,
	pub last_time: f32,
	pub origin: f32,
	pub ready: bool,
	pub yaw: Quat,
	pub tint: Vec4,
	pub slots: Vec<Entity>,
	pub grains: grains::GrainSimulation,
}
#[derive(Component)]
struct ElementSlot {
	index: usize,
	tag: usize,
}
#[derive(Resource, Default)]
struct OwnedSlots(std::collections::HashMap<Entity, usize>);
#[derive(Resource, Default)]
struct FrameCache(
	std::collections::HashMap<(Entity, bevy::asset::AssetId<ExplosionTrack>), (f32, Frame)>,
);
/// Explicit admission limits, never substitutes styles or queues a delayed effect.
#[derive(Resource)]
pub struct ExplosionLimits {
	pub cartoon: usize,
	pub combined: usize,
}
impl Default for ExplosionLimits {
	fn default() -> Self {
		Self { cartoon: 8, combined: 3 }
	}
}
#[derive(Resource, Default, Debug)]
pub struct ExplosionDiagnostics {
	pub rejected: usize,
	pub last_error: Option<String>,
}
fn placement(spawn: &VfxSpawn, heading: Option<Vec3>) -> Result<Quat, &'static str> {
	if !spawn.scale.is_finite()
		|| !spawn.playback.is_finite()
		|| !spawn.transform.scale.is_finite()
		|| !spawn.transform.translation.is_finite()
		|| (spawn.transform.rotation * Vec3::Y - Vec3::Y).length() > 1e-4
		|| !spawn.transform.rotation.is_finite()
		|| (spawn.transform.rotation.length() - 1.).abs() > 1e-4
		|| (spawn.transform.scale - Vec3::ONE * spawn.transform.scale.x)
			.abs()
			.max_element()
			> 1e-5
		|| spawn.transform.scale.min_element() <= 0.
	{
		return Err(
			"explosion requires finite translation, upright yaw and positive uniform spawn scale",
		);
	}
	if let Some(h) = heading {
		if !h.is_finite() || h.y.abs() > 1e-4 || h.length_squared() < 1e-8 {
			return Err("explosion heading must be finite, nonzero, horizontal");
		}
		let yaw = Quat::from_rotation_y(-h.z.atan2(h.x));
		Ok(spawn.transform.rotation.inverse() * yaw)
	} else {
		let mut s = spawn.resolved_seed();
		s ^= s >> 30;
		s = s.wrapping_mul(0xbf58476d1ce4e5b9);
		s ^= s >> 27;
		Ok(Quat::from_rotation_y((s as u32 as f32 / u32::MAX as f32) * std::f32::consts::TAU))
	}
}
impl CartoonExplosionPart {
	pub fn load(s: &AssetServer) -> Self {
		Self {
			track: s.load("vfx/cartoon_explosion/cartoon_explosion.explosion.json"),
			heading: None,
			lod: default(),
		}
	}
	pub(crate) fn spawn(
		&self,
		c: &mut Commands,
		parent: Entity,
		transform: Transform,
		s: &VfxSpawn,
	) {
		spawn_layer(c, parent, transform, s, self.track.clone(), None, self.heading, self.lod);
	}
}
impl CombinedExplosionPart {
	pub fn load(s: &AssetServer) -> Self {
		Self {
			asset: s.load("vfx/combined_explosion/combined_explosion.combined.json"),
			heading: None,
			quality: default(),
			diagnostic: default(),
		}
	}
	pub(crate) fn spawn(
		&self,
		c: &mut Commands,
		parent: Entity,
		transform: Transform,
		s: &VfxSpawn,
	) {
		spawn_layer(
			c,
			parent,
			transform,
			s,
			default(),
			Some(self.clone()),
			self.heading,
			default(),
		);
	}
}
fn spawn_layer(
	c: &mut Commands,
	parent: Entity,
	mut transform: Transform,
	s: &VfxSpawn,
	track: Handle<ExplosionTrack>,
	combined: Option<CombinedExplosionPart>,
	heading: Option<Vec3>,
	lod: ExplosionLod,
) {
	let yaw = match placement(s, heading) {
		Ok(q) => q,
		Err(e) => {
			let e = e.to_owned();
			c.queue(move |w: &mut World| {
				let mut d = w.resource_mut::<ExplosionDiagnostics>();
				d.rejected += 1;
				d.last_error = Some(e);
				// A rejected standalone recipe must not retain an invalid host clock.
				let rejected_host = w.get::<VfxInstance>(parent).is_some_and(|i| {
					matches!(i.name.as_str(), "cartoon-explosion" | "combined-explosion")
						|| !i.playback.is_finite()
				});
				if rejected_host {
					w.despawn(parent);
				}
			});
			return;
		}
	};
	transform.rotation = yaw;
	transform.scale *= s.transform.scale.x;
	let id = c
		.spawn((
			ChildOf(parent),
			VfxMemberOf(parent),
			transform,
			Visibility::Visible,
			ExplosionLayer {
				track,
				combined,
				lod,
				frame: default(),
				last_time: -1.,
				origin: 0.,
				ready: false,
				yaw,
				tint: s.tint_or_white().to_linear().to_vec4(),
				slots: Vec::new(),
				grains: default(),
			},
			VfxLayerLife {
				age: 0.,
				duration: 14.,
				playback: s.clamped_playback(),
				waiting_for_emitter: true,
			},
		))
		.id();
	c.queue(move |w: &mut World| {
		let origin = w.get::<VfxInstance>(parent).map(|i| i.age).unwrap_or(0.);
		if let Some(mut layer) = w.get_mut::<ExplosionLayer>(id) {
			layer.origin = origin;
		}
	});
}
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SmokeExplosionTick;
pub struct SmokeExplosionPlugin;
impl Plugin for SmokeExplosionPlugin {
	fn build(&self, app: &mut App) {
		app.init_asset::<ExplosionTrack>()
			.init_asset_loader::<track::TrackLoader>()
			.init_asset::<density::CombinedExplosionAsset>()
			.init_asset_loader::<density::CombinedLoader>()
			.init_resource::<OwnedSlots>()
			.init_resource::<FrameCache>()
			.init_resource::<ExplosionLimits>()
			.init_resource::<ExplosionDiagnostics>();
		cartoon::plugin(app);
		render::plugin(app);
		app.add_systems(
			Update,
			tick.in_set(SmokeExplosionTick).after(crate::lifecycle::tick_vfx_instances),
		);
	}
}
fn tick(
	mut commands: Commands,
	server: Res<AssetServer>,
	tracks: Res<Assets<ExplosionTrack>>,
	assets: Res<Assets<density::CombinedExplosionAsset>>,
	instances: Query<(&VfxInstance, &GlobalTransform)>,
	mut layers: Query<(
		Entity,
		&VfxMemberOf,
		&GlobalTransform,
		&mut ExplosionLayer,
		&mut VfxLayerLife,
	)>,
	mut slots: Query<(&ElementSlot, &mut Transform, &mut Visibility)>,
	mut pool: ResMut<cartoon::Pool>,
	mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
	mut owned: ResMut<OwnedSlots>,
	mut cache: ResMut<FrameCache>,
	limits: Res<ExplosionLimits>,
	mut diagnostics: ResMut<ExplosionDiagnostics>,
	gpu: Option<Res<render::GpuReadiness>>,
	cameras: Query<Has<bevy::core_pipeline::prepass::DepthPrepass>, With<Camera3d>>,
) {
	let mut dirty = false;
	cache.0.retain(|(root, _), _| instances.get(*root).is_ok());
	let mut cartoons =
		layers.iter().filter(|(_, _, _, l, _)| l.ready && l.combined.is_none()).count();
	let mut combined =
		layers.iter().filter(|(_, _, _, l, _)| l.ready && l.combined.is_some()).count();
	let dead = owned
		.0
		.keys()
		.filter(|id| layers.get(**id).is_err())
		.copied()
		.collect::<Vec<_>>();
	for id in dead {
		owned.0.remove(&id);
	}
	for (entity, member, world, mut layer, mut life) in &mut layers {
		let Ok((instance, _)) = instances.get(member.0) else {
			continue;
		};
		if !layer.ready {
			if let Some(part) = &layer.combined {
				if !server.is_loaded_with_dependencies(part.asset.id()) {
					if server.load_state(part.asset.id()).is_failed()
						|| matches!(
							server.get_recursive_dependency_load_state(part.asset.id()),
							Some(bevy::asset::RecursiveDependencyLoadState::Failed(_))
						) {
						diagnostics.last_error =
							Some("Combined explosion dependency failed".into());
						commands.entity(entity).try_despawn();
					}
					continue;
				}
				let Some(asset) = assets.get(&part.asset) else {
					continue;
				};
				layer.track = asset.track.clone();
			}
			if !server.is_loaded_with_dependencies(layer.track.id()) {
				if server.load_state(layer.track.id()).is_failed()
					|| matches!(
						server.get_recursive_dependency_load_state(layer.track.id()),
						Some(bevy::asset::RecursiveDependencyLoadState::Failed(_))
					) {
					diagnostics.last_error = Some("Cartoon explosion dependency failed".into());
					commands.entity(entity).try_despawn();
				}
				continue;
			}
			if layer.combined.is_some()
				&& (cameras.is_empty() || cameras.iter().any(|depth| !depth))
			{
				diagnostics.rejected += 1;
				diagnostics.last_error =
					Some("CombinedExplosion requires DepthPrepass on every preview camera".into());
				warn!("CombinedExplosion spawn rejected: missing camera DepthPrepass");
				commands.entity(entity).try_despawn();
				continue;
			}
			let reject = if layer.combined.is_some() {
				combined >= limits.combined.min(3)
			} else {
				cartoons >= limits.cartoon.min(8)
			};
			if reject {
				diagnostics.rejected += 1;
				warn!("authored explosion spawn rejected: concurrency cap reached");
				diagnostics.last_error =
					Some("explosion concurrency cap reached; spawn rejected".into());
				commands.entity(entity).try_despawn();
				continue;
			}
			let Some(track) = tracks.get(&layer.track) else {
				continue;
			};
			if layer.combined.is_none() {
				cartoons += 1;
				let block = (0..8).find(|b| !owned.0.values().any(|v| v == b)).unwrap_or(0);
				for (i, mesh) in track.manifest.elements.iter().enumerate() {
					let slot = commands
						.spawn((
							ChildOf(entity),
							Mesh3d(track.meshes[mesh.mesh].clone()),
							MeshMaterial3d(pool.material.clone()),
							bevy::mesh::MeshTag::new((block * 349 + i) as u32),
							Transform::IDENTITY,
							Visibility::Hidden,
							NotShadowCaster,
							ElementSlot { index: i, tag: block * 349 + i },
						))
						.id();
					layer.slots.push(slot);
				}
				owned.0.insert(entity, block);
			} else {
				combined += 1;
			}
			layer.ready = true;
			life.waiting_for_emitter = layer.combined.is_some();
		}
		if layer.combined.is_some() && life.waiting_for_emitter {
			if gpu.as_ref().is_some_and(|r| r.contains(entity)) {
				life.waiting_for_emitter = false;
			} else {
				if layer.last_time < 0. {
					if let Some(track) = tracks.get(&layer.track) {
						track.evaluate(0., &mut layer.frame);
						layer.last_time = 0.;
					}
				}
				continue;
			}
		}
		if !instance.armed {
			continue;
		}
		let t = (instance.age - layer.origin).max(0.).min(14.);
		life.age = t;
		if t >= 14. {
			commands.entity(entity).try_despawn();
			continue;
		}
		if t == layer.last_time {
			continue;
		}
		layer.last_time = t;
		let Some(track) = tracks.get(&layer.track) else {
			continue;
		};
		let cached = cache
			.0
			.entry((member.0, layer.track.id()))
			.or_insert_with(|| (-1., Frame::default()));
		if cached.0 != t {
			track.evaluate(t, &mut cached.1);
			cached.0 = t;
		}
		layer.frame.clone_from(&cached.1);
		if let Some(part) = &layer.combined {
			if let Some(asset) = assets.get(&part.asset) {
				layer.grains.advance(t, &asset.grains);
			}
			continue;
		}
		let effect = world.to_matrix();
		let rotation = world.rotation();
		for id in &layer.slots {
			let Ok((slot, mut transform, mut visible)) = slots.get_mut(*id) else {
				continue;
			};
			let state = layer.frame.states.iter().find(|s| s.id == slot.index);
			let e = &track.manifest.elements[slot.index];
			let rank = match layer.lod {
				ExplosionLod::Full => 136,
				ExplosionLod::Medium => 60,
				ExplosionLod::Low => 25,
			};
			let Some(s) = state.filter(|s| {
				s.visible() && (e.kind == "spike" || e.lod_rank.is_some_and(|r| r < rank))
			}) else {
				if *visible != Visibility::Hidden {
					*visible = Visibility::Hidden;
				}
				continue;
			};
			if *visible != Visibility::Visible {
				*visible = Visibility::Visible;
			}
			let k = match layer.lod {
				ExplosionLod::Full => 1.,
				ExplosionLod::Medium => e.lod_scale.medium,
				ExplosionLod::Low => e.lod_scale.low,
			};
			*transform =
				Transform { translation: s.position, rotation: s.rotation, scale: s.scale * k };
			let centroid = e
				.lobe
				.map(|l| effect.transform_point3(layer.frame.centroids[l]))
				.unwrap_or(effect.transform_point3(s.position));
			let data = Surface {
				centroid: centroid.extend(0.),
				opening: (rotation * s.open_dir).extend(s.openness),
				controls: Vec4::new(
					s.dissolve,
					s.tone,
					layer.frame.mass_height,
					if e.kind == "spike" { 1. } else { 0. },
				),
				effect_from_world: effect.inverse(),
				tint: layer.tint,
			};
			let rows = data.rows();
			let base = slot.tag * 8;
			if pool.values[base..base + 8] != rows {
				pool.values[base..base + 8].copy_from_slice(&rows);
				dirty = true;
			}
		}
	}
	if dirty {
		let handle = pool.buffer.clone();
		if let Some(mut buffer) = buffers.get_mut(&handle) {
			buffer.clear();
			buffer.extend_from_slice(&pool.values);
		}
	}
}
