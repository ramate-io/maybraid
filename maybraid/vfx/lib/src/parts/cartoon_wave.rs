//! Finite exterior rolling wave. Rendering is shared with Cartoon landing.
pub mod motion;
use crate::{
	composition::{CartoonLandingPart, CartoonWavePart},
	membership::VfxMemberOf,
	parts::cartoon_landing::LandingMaterial,
	spawn::{VfxInstance, VfxLayerLife, VfxSpawn},
};
use bevy::{asset::AssetId, camera::primitives::Aabb, mesh::VertexAttributeValues, prelude::*};
use std::sync::Arc;

#[derive(Component)]
pub struct RollingWave {
	part: CartoonWavePart,
	wave: Option<motion::Wave>,
	age: Option<f32>,
	start: f32,
	poses: [Option<motion::Pose>; 15],
}
#[derive(Component)]
struct Body {
	controller: Entity,
	index: usize,
	material: Handle<LandingMaterial>,
}
/// Three immutable position arrays shared across all instances, without GPU mesh copies.
#[derive(Resource, Default)]
pub(crate) struct Geometry {
	slots: Vec<(AssetId<Mesh>, Arc<[Vec3]>)>,
}
impl Geometry {
	pub(crate) fn load(
		&mut self,
		handles: &[Handle<Mesh>; 3],
		meshes: &Assets<Mesh>,
	) -> Option<[Arc<[Vec3]>; 3]> {
		let mut out = Vec::new();
		for handle in handles {
			if let Some((_, points)) = self.slots.iter().find(|(id, _)| *id == handle.id()) {
				out.push(points.clone());
				continue;
			}
			let mesh = meshes.get(handle)?;
			let VertexAttributeValues::Float32x3(p) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
			else {
				return None;
			};
			if p.is_empty() || self.slots.len() >= 3 {
				return None;
			}
			let points: Arc<[Vec3]> = p.iter().copied().map(Vec3::from_array).collect();
			self.slots.push((handle.id(), points.clone()));
			out.push(points);
		}
		out.try_into().ok()
	}
}
impl CartoonWavePart {
	pub fn load(server: &AssetServer) -> Self {
		Self { meshes: CartoonLandingPart::load(server).meshes }
	}
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) {
		commands.spawn((
			ChildOf(parent),
			VfxMemberOf(parent),
			transform,
			Visibility::Visible,
			RollingWave { part: self.clone(), wave: None, age: None, start: 0., poses: [None; 15] },
			VfxLayerLife {
				age: 0.,
				duration: 4.,
				playback: spawn.clamped_playback(),
				waiting_for_emitter: true,
			},
		));
	}
}
pub struct WavePlugin;
impl Plugin for WavePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<Geometry>()
			.add_systems(Update, tick.after(crate::lifecycle::tick_vfx_instances));
	}
}
fn tick(
	mut commands: Commands,
	server: Res<AssetServer>,
	meshes: Res<Assets<Mesh>>,
	mut geometry: ResMut<Geometry>,
	mut materials: ResMut<Assets<LandingMaterial>>,
	instances: Query<&VfxInstance>,
	mut layers: Query<(Entity, &VfxMemberOf, &mut RollingWave, &mut VfxLayerLife)>,
	mut bodies: Query<(&Body, &mut Transform, &mut Visibility)>,
) {
	let mut admitted = layers.iter().filter(|(_, _, l, _)| l.wave.is_some()).count();
	let mut order = layers.iter().map(|(e, _, _, _)| e).collect::<Vec<_>>();
	order.sort();
	for id in order {
		let Ok((entity, member, mut layer, mut life)) = layers.get_mut(id) else {
			continue;
		};
		let Ok(instance) = instances.get(member.root()) else {
			continue;
		};
		if layer.wave.is_none() {
			if admitted >= 16 {
				warn!("Cartoon wave rejected: sixteen concurrent waves already active");
				commands.entity(member.root()).try_despawn();
				continue;
			}
			if layer.part.meshes.iter().any(|h| {
				server
					.get_load_states(h.id())
					.is_some_and(|(s, _, _)| matches!(s, bevy::asset::LoadState::Failed(_)))
			}) {
				warn!("Cartoon wave mesh load failed; cancelling event");
				commands.entity(member.root()).try_despawn();
				continue;
			}
			if !layer.part.meshes.iter().all(|h| meshes.contains(h)) {
				continue;
			}
			let Some(rest) = geometry.load(&layer.part.meshes, &meshes) else {
				warn!("Cartoon wave requires three nonempty Float32x3 authored position arrays; cancelling event");
				commands.entity(member.root()).try_despawn();
				continue;
			};
			layer.wave = Some(motion::Wave::new(rest));
			layer.start = instance.age;
			for index in 0..15 {
				let variant = if index < 12 { index % 3 } else { 0 };
				// All selected child parents (3,0,9) use Shoulder, as in the delivery.
				let material = materials.add(LandingMaterial::wave(Vec4::ZERO));
				commands.spawn((
					ChildOf(entity),
					VfxMemberOf(member.root()),
					Mesh3d(layer.part.meshes[variant].clone()),
					MeshMaterial3d(material.clone()),
					Transform::IDENTITY,
					Visibility::Hidden,
					// Delivered conservative DEFORMED local domain. Entity transforms supply tilt,
					// scale and pivot translation; this replaces automatic rest-mesh bounds.
					Aabb::from_min_max(
						Vec3::new(-2.042, -0.444, -1.817),
						Vec3::new(1.917, 3.531, 1.786),
					),
					Body { controller: entity, index, material },
				));
			}
			admitted += 1;
			life.waiting_for_emitter = false;
		}
		let local_age = (instance.age - layer.start).max(0.);
		if instance.armed && layer.age != Some(local_age) {
			if let Some(wave) = &layer.wave {
				layer.poses = wave.states(local_age);
			}
			layer.age = Some(local_age);
		}
		life.age = local_age;
	}
	for (body, mut transform, mut visible) in &mut bodies {
		let Ok((_, member, layer, _)) = layers.get(body.controller) else {
			continue;
		};
		let armed = instances.get(member.root()).is_ok_and(|i| i.armed);
		if let Some(pose) = layer.poses[body.index].filter(|_| armed) {
			*transform = pose.transform();
			*visible = Visibility::Visible;
			if let Some(mut m) = materials.get_mut(&body.material) {
				if m.cutoff != pose.coefficients.0 {
					m.set_wave_coefficients(pose.coefficients.0);
				}
			}
		} else {
			*visible = Visibility::Hidden;
		}
	}
}
#[cfg(test)]
pub(crate) mod tests;
