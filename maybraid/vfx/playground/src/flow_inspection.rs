//! Optional bounded validation overlays; never part of the dust recipe.
use bevy::prelude::*;
use maybraid_vfx::parts::combined::{
	flow::{H, N, ORIGIN},
	CombinedState,
};
#[derive(Resource, Default)]
pub struct FlowInspection(pub String);
pub fn draw(view: Res<FlowInspection>, states: Query<&CombinedState>, mut gizmos: Gizmos) {
	if view.0.is_empty() || view.0 == "normal" {
		return;
	}
	for state in &states {
		let Some(i) = &state.simulation.interactive else {
			continue;
		};
		let Some(f) = &i.flow else {
			continue;
		};
		if view.0 == "probes" {
			for p in i.puffs.iter().flatten() {
				let c = Vec3::new(0.05, 0.66, 0.);
				for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
					let a = p.model().transform_point3(p.pose.coefficients.map(c - axis * 0.45));
					let b = p.model().transform_point3(p.pose.coefficients.map(c + axis * 0.45));
					gizmos.line(a, b, Color::srgb(0., 1., 1.));
				}
			}
			continue;
		}
		for z in (0..N[2]).step_by(2) {
			for y in (0..N[1]).step_by(2) {
				for x in (0..N[0]).step_by(2) {
					let at = ORIGIN
						+ (bevy::math::DVec3::new(x as f64, y as f64, z as f64)
							+ bevy::math::DVec3::splat(0.5))
							* H;
					let start = at.as_vec3();
					if view.0 == "solids" {
						if f.solid_at(at) {
							gizmos.line(
								start - Vec3::Y * 0.08,
								start + Vec3::Y * 0.08,
								Color::srgb(1., 0., 1.),
							);
						}
					} else if view.0.starts_with("divergence") {
						let index = x + N[0] * (y + N[1] * z);
						let d = if view.0 == "divergence-before" {
							f.unprojected[index]
						} else {
							f.divergence[index]
						};
						gizmos.line(
							start,
							start + Vec3::Y * (d as f32 * 0.05).clamp(-0.2, 0.2),
							Color::srgb(1., 0.4, 0.1),
						);
					} else {
						gizmos.arrow(
							start,
							start + f.sample(at).as_vec3() * 0.15,
							Color::srgb(0.2, 0.8, 1.),
						);
					}
				}
			}
		}
	}
}
