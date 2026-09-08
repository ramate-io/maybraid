//! Wireframe motor / head / headshot band on the test dummy.

use bevy::prelude::*;
use player::LocomotionCapsule;

use crate::damage::HeadshotBand;
use crate::session::{RangeSession, TestDummy};

const MOTOR: Color = Color::srgb(0.95, 0.72, 0.2);
const HEAD: Color = Color::srgb(0.25, 0.85, 1.0);
const BAND: Color = Color::srgb(0.55, 0.7, 1.0);

pub(crate) fn draw_dummy_hit_volumes(
	session: Res<RangeSession>,
	mut gizmos: Gizmos,
	dummies: Query<(&GlobalTransform, &LocomotionCapsule, Option<&HeadshotBand>), With<TestDummy>>,
) {
	if !session.is_test_dummy() {
		return;
	}
	for (transform, hull, band) in &dummies {
		let origin = transform.translation();
		draw_vertical_capsule(&mut gizmos, origin, hull.radius, hull.length, MOTOR);
		if let Some(head) = hull.head_capsule() {
			let center = transform.transform_point(head.local_transform().translation);
			gizmos.primitive_3d(
				&Capsule3d::new(head.radius, head.length),
				Isometry3d::from_translation(center),
				HEAD,
			);
		}
		if let Some(band) = band {
			let plane = transform.transform_point(Vec3::Y * band.min_local_y);
			let iso = Isometry3d::from_translation(plane)
				* Isometry3d::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
			gizmos.circle(iso, hull.radius + 0.08, BAND);
		}
	}
}

fn draw_vertical_capsule(
	gizmos: &mut Gizmos,
	origin: Vec3,
	radius: f32,
	length: f32,
	color: Color,
) {
	let half = length * 0.5;
	gizmos.sphere(Isometry3d::from_translation(origin + Vec3::Y * half), radius, color);
	gizmos.sphere(Isometry3d::from_translation(origin - Vec3::Y * half), radius, color);
	gizmos.line(origin + Vec3::Y * half, origin - Vec3::Y * half, color);
}
