//! CPU mirror of `hash.wesl`.

use bevy::math::{IVec2, Vec2};

/// Hash helpers matching the WESL vegetation library bit-for-bit.
pub struct VegetationHash;

impl VegetationHash {
	pub fn hash_u32(value: u32) -> u32 {
		hash_u32(value)
	}

	pub fn plant_id(patch_seed: u32, local_index: u32) -> u32 {
		plant_id(patch_seed, local_index)
	}

	pub fn density_rank(id: u32) -> f32 {
		density_rank(id)
	}

	pub fn value_noise_2(p: Vec2) -> f32 {
		value_noise_2(p)
	}
}

pub fn hash_u32(value: u32) -> u32 {
	let mut state = value;
	state ^= 2_747_636_419;
	state = state.wrapping_mul(2_654_435_769);
	state ^= state >> 16;
	state = state.wrapping_mul(2_654_435_769);
	state ^= state >> 16;
	state = state.wrapping_mul(2_654_435_769);
	state
}

pub fn hash_u32_to_unit(value: u32) -> f32 {
	hash_u32(value) as f32 * (1.0 / 4_294_967_295.0)
}

pub fn plant_id(patch_seed: u32, local_index: u32) -> u32 {
	hash_u32(patch_seed ^ hash_u32(local_index ^ 0x9e3779b9))
}

pub fn density_rank(id: u32) -> f32 {
	hash_u32_to_unit(id ^ 0x85ebca6b)
}

pub fn hash_lattice(cell: IVec2) -> f32 {
	let x = (cell.x as u32).wrapping_mul(374_761_393);
	let y = (cell.y as u32).wrapping_mul(668_265_263);
	hash_u32_to_unit(x ^ y ^ 0x9e3779b9)
}

pub fn value_noise_2(p: Vec2) -> f32 {
	let cell = IVec2::new(p.x.floor() as i32, p.y.floor() as i32);
	let f = Vec2::new(p.x - p.x.floor(), p.y - p.y.floor());
	let u = f * f * (Vec2::splat(3.0) - 2.0 * f);
	let a = hash_lattice(cell);
	let b = hash_lattice(cell + IVec2::new(1, 0));
	let c = hash_lattice(cell + IVec2::new(0, 1));
	let d = hash_lattice(cell + IVec2::new(1, 1));
	let x0 = a + (b - a) * u.x;
	let x1 = c + (d - c) * u.x;
	x0 + (x1 - x0) * u.y
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plant_identity_is_stable_and_independent_of_order() -> anyhow::Result<()> {
		let a = VegetationHash::plant_id(7, 3);
		let b = VegetationHash::plant_id(7, 3);
		let c = VegetationHash::plant_id(7, 4);
		assert_eq!(a, b);
		assert_ne!(a, c);
		assert_ne!(VegetationHash::plant_id(8, 3), a);
		Ok(())
	}

	#[test]
	fn density_rank_is_unit_interval() -> anyhow::Result<()> {
		for i in 0..256u32 {
			let rank = VegetationHash::density_rank(VegetationHash::plant_id(1, i));
			assert!((0.0..1.0).contains(&rank), "rank {rank} out of range");
		}
		Ok(())
	}

	#[test]
	fn value_noise_is_stable_and_bounded() -> anyhow::Result<()> {
		let a = VegetationHash::value_noise_2(Vec2::new(-12.4, 8.1));
		let b = VegetationHash::value_noise_2(Vec2::new(-12.4, 8.1));
		assert_eq!(a, b);
		assert!((0.0..1.0).contains(&a));
		let far = VegetationHash::value_noise_2(Vec2::new(30.0, -4.0));
		assert!((a - far).abs() > 1e-4);
		Ok(())
	}
}
