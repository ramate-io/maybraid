//! Emissive world reticle at the first fixed hit along the firearm barrel.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use firearms::{muzzle_world, BoneMap, FirearmMembers, FirearmRoot, RigRoot};
use lod_avian::PhysicsInteractionLayer;

use crate::character::HeldFirearm;

const AIM_DISTANCE: f32 = 100.0;
const SURFACE_LIFT: f32 = 0.015;
const ANGULAR_SIZE: f32 = 0.004;

#[derive(Component)]
pub(crate) struct Reticle;

pub(crate) fn spawn_reticle(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let color = Color::srgb(0.45, 1.0, 0.95);
	let glow = color.to_linear();
	commands.spawn((
		Name::new("reticle"),
		Reticle,
		Mesh3d(meshes.add(Sphere::new(1.0))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: color,
			emissive: LinearRgba::rgb(glow.red * 12.0, glow.green * 12.0, glow.blue * 12.0),
			unlit: true,
			depth_bias: -10.0,
			..default()
		})),
		Transform::IDENTITY,
		Visibility::Hidden,
		NotShadowCaster,
	));
}

/// Cast from the barrel and place the marker just in front of the first surface.
pub(crate) fn update_reticle(
	spatial: SpatialQuery,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	guns: Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform, Without<Camera3d>>,
	mut reticles: Query<(&mut Transform, &mut Visibility), With<Reticle>>,
) {
	let Ok(camera) = cameras.single() else {
		return;
	};
	let Some((origin, direction)) = barrel_ray(&guns, &maps, &globals) else {
		return;
	};
	let Ok((mut transform, mut visibility)) = reticles.single_mut() else {
		return;
	};

	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	let (target, normal) =
		if let Some(hit) = spatial.cast_ray(origin, direction, AIM_DISTANCE, true, &filter) {
			(origin + *direction * hit.distance, hit.normal)
		} else {
			(origin + *direction * AIM_DISTANCE, -*direction)
		};
	let target = target + normal * SURFACE_LIFT;
	let distance = camera.translation().distance(target);
	*transform = Transform::from_translation(target)
		.with_scale(Vec3::splat((distance * ANGULAR_SIZE).clamp(0.025, 0.3)));
	*visibility = Visibility::Visible;
}

fn barrel_ray(
	guns: &Query<&FirearmMembers, (With<HeldFirearm>, With<FirearmRoot>)>,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &Query<&GlobalTransform, Without<Camera3d>>,
) -> Option<(Vec3, Dir3)> {
	let members = guns.single().ok()?;
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&barrel) = map.by_name.get("barrel") else {
			continue;
		};
		let Ok(global) = globals.get(barrel) else {
			continue;
		};
		let (muzzle, direction) = muzzle_world(global);
		let Ok(direction) = Dir3::new(direction) else {
			continue;
		};
		return Some((muzzle, direction));
	}
	None
}
