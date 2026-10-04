//! Shared lobe mesh and age-driven cluster animation.

use bevy::prelude::*;

use crate::composition::LobeSpec;

/// Low-poly icosphere with vertex colors holding object-space positions.
pub fn rounded_lobe_mesh() -> Mesh {
	let mut mesh = Sphere::new(0.5).mesh().ico(2).expect("lobe icosphere");
	let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
	else {
		return mesh;
	};
	let colors: Vec<[f32; 4]> = positions.iter().map(|p| [p[0], p[1], p[2], 1.0]).collect();
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
	mesh
}

pub fn hash_u32(x: u32) -> u32 {
	let mut x = x.wrapping_mul(0x45d9f3bb);
	x = (x ^ (x >> 16)).wrapping_mul(0x45d9f3bb);
	x ^ (x >> 16)
}

/// Seeded offset in `[-1, 1]^3` so simultaneous instances do not match.
pub fn lobe_jitter(seed: u64, index: u32) -> Vec3 {
	let h = hash_u32((seed as u32).wrapping_add(index.wrapping_mul(0x9e3779b9)));
	Vec3::new(
		((h & 0xff) as f32 / 127.5) - 1.0,
		(((h >> 8) & 0xff) as f32 / 127.5) - 1.0,
		(((h >> 16) & 0xff) as f32 / 127.5) - 1.0,
	)
}

/// Fast initial swell, slower later motion. Shared so lobes read as one blast.
pub fn expand_amount(age: f32, duration: f32) -> f32 {
	let t = if duration > 1e-4 { (age / duration).clamp(0.0, 1.0) } else { 1.0 };
	1.0 - (1.0 - t).powf(2.35)
}

pub fn lobe_transform(spec: &LobeSpec, age: f32, seed: u64, index: u32) -> Transform {
	let expand = expand_amount(age, spec.duration);
	let jitter = lobe_jitter(seed, index) * 0.035;
	let offset = spec.offset + jitter;
	let rest = Quat::from_euler(EulerRot::YXZ, spec.euler.y, spec.euler.x, spec.euler.z);
	let spin = Quat::from_axis_angle(rest * Vec3::X, spec.roll * expand);
	let stretch = Vec3::new(1.0, 1.0 + expand * 0.18, 1.0);
	Transform {
		translation: offset * (1.0 + expand * 0.38) + Vec3::Y * spec.rise * expand,
		rotation: spin * rest,
		scale: spec.scale * stretch * (1.0 + spec.expand * expand),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn expansion_is_fast_then_slow() {
		assert!(expand_amount(0.1, 1.0) > 0.2);
		assert!(expand_amount(0.1, 1.0) < expand_amount(0.5, 1.0));
		assert!((expand_amount(1.0, 1.0) - 1.0).abs() < 1e-4);
	}

	#[test]
	fn seeds_jitter_independently() {
		let a = lobe_jitter(1, 0);
		let b = lobe_jitter(2, 0);
		assert!(a.distance(b) > 1e-4);
	}

	#[test]
	fn lobes_rotate_as_they_expand() {
		let spec = LobeSpec {
			offset: Vec3::ZERO,
			scale: Vec3::new(0.7, 0.4, 0.5),
			euler: Vec3::new(0.2, 0.4, -0.1),
			expand: 0.8,
			rise: 0.2,
			roll: 1.4,
			duration: 1.0,
		};
		let early = lobe_transform(&spec, 0.05, 1, 0);
		let late = lobe_transform(&spec, 0.8, 1, 0);
		assert!(early.rotation.angle_between(late.rotation) > 0.2);
		assert!((early.scale.x - early.scale.y).abs() > 0.05);
	}
}
