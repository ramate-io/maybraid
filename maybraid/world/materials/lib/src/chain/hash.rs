//! CPU port of [`hash.wgsl`](hash.wgsl).
//!
//! WGSL `fract` is `x - floor(x)` for every sign. Rust's [`f32::fract`] is
//! `x - trunc(x)` and disagrees on negatives, so this module uses
//! [`wgsl_fract`]. Tests lock the WGSL source and these constants together
//! and check golden vectors computed both from this port and from a
//! transcription of the WGSL function text.

/// Scale inside `hash13`: `fract(p * vec3(0.1031, 0.1030, 0.0973))`.
pub const HASH13_SCALE: [f32; 3] = [0.1031, 0.1030, 0.0973];
/// Offset inside `hash13`: `dot(p3, p3.yzx + vec3(33.33))`.
pub const HASH13_OFFSET: f32 = 33.33;
/// `hash21(p)` is `hash13(vec3(p, 17.13))`.
pub const HASH21_Z: f32 = 17.13;
/// `link_weakness` x: `id * 17.13`.
pub const WEAKNESS_ID_SCALE: f32 = 17.13;
/// `link_weakness` y: `seed * 0.013`.
pub const WEAKNESS_SEED_SCALE: f32 = 0.013;
/// `link_weakness` z: `4.2`.
pub const WEAKNESS_Z: f32 = 4.2;

/// GPU source of truth for [`hash13`], [`hash21`], and [`link_weakness`].
#[cfg(test)]
const HASH_WGSL: &str = include_str!("hash.wgsl");

/// Fragment shader that must keep the same hash function bodies as [`HASH_WGSL`].
#[cfg(test)]
const CHAIN_MATERIAL_WGSL: &str = include_str!("chain_material.wgsl");

/// WGSL `fract`: always toward +∞ from `floor`, including for negatives.
#[inline]
pub fn wgsl_fract(x: f32) -> f32 {
	x - x.floor()
}

/// Dave Hoskins `hash13`, matching `hash.wgsl`.
pub fn hash13(p: [f32; 3]) -> f32 {
	let p3 = [
		wgsl_fract(p[0] * HASH13_SCALE[0]),
		wgsl_fract(p[1] * HASH13_SCALE[1]),
		wgsl_fract(p[2] * HASH13_SCALE[2]),
	];
	// `dot(p3, p3.yzx + vec3(33.33))`
	let d = p3[0] * (p3[1] + HASH13_OFFSET)
		+ p3[1] * (p3[2] + HASH13_OFFSET)
		+ p3[2] * (p3[0] + HASH13_OFFSET);
	wgsl_fract((p3[0] + p3[1]) * p3[2] + d)
}

/// `hash13(vec3(p, 17.13))`.
pub fn hash21(p: [f32; 2]) -> f32 {
	hash13([p[0], p[1], HASH21_Z])
}

/// Which links give way first. Higher is weaker. Same call as the shader.
pub fn link_weakness(link_id: f32, seed: f32) -> f32 {
	hash13([link_id * WEAKNESS_ID_SCALE, seed * WEAKNESS_SEED_SCALE, WEAKNESS_Z])
}

#[cfg(test)]
mod tests {
	use super::*;

	const WEAKNESS_GOLDEN_INPUTS: &[(f32, f32)] =
		&[(0.0, 0.0), (3.0, 11.0), (4.0, 11.0), (3.0, 29.0), (13.0, -7.0)];

	const HASH13_GOLDEN_INPUTS: &[[f32; 3]] =
		&[[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [3.0 * 17.13, 11.0 * 0.013, 4.2], [-2.5, 0.5, 1.25]];

	fn hash13_from_wgsl_source(p: [f32; 3]) -> f32 {
		let scale = parse_vec3_literal(HASH_WGSL, "p * vec3<f32>(")
			.expect("hash.wgsl hash13 scale literal");
		let offset = parse_f32_after(HASH_WGSL, "p3.yzx + vec3<f32>(")
			.expect("hash.wgsl hash13 offset literal");
		let p3 =
			[wgsl_fract(p[0] * scale[0]), wgsl_fract(p[1] * scale[1]), wgsl_fract(p[2] * scale[2])];
		let d = p3[0] * (p3[1] + offset) + p3[1] * (p3[2] + offset) + p3[2] * (p3[0] + offset);
		wgsl_fract((p3[0] + p3[1]) * p3[2] + d)
	}

	fn link_weakness_from_wgsl_source(link_id: f32, seed: f32) -> f32 {
		let id_scale = parse_f32_after(HASH_WGSL, "id * ").expect("hash.wgsl id scale");
		let seed_scale = parse_f32_after(HASH_WGSL, "seed * ").expect("hash.wgsl seed scale");
		let z = parse_f32_after(HASH_WGSL, "seed * 0.013, ").expect("hash.wgsl weakness z");
		hash13_from_wgsl_source([link_id * id_scale, seed * seed_scale, z])
	}

	fn parse_vec3_literal(src: &str, marker: &str) -> Option<[f32; 3]> {
		let rest = src.split_once(marker)?.1;
		let inside = rest.split_once(')')?.0;
		let mut parts = inside.split(',').map(|s| s.trim().parse::<f32>().ok());
		Some([parts.next()??, parts.next()??, parts.next()??])
	}

	fn parse_f32_after(src: &str, marker: &str) -> Option<f32> {
		let rest = src.split_once(marker)?.1.trim_start();
		let token: String = rest
			.chars()
			.take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+')
			.collect();
		token.parse().ok()
	}

	fn wgsl_fn_body<'a>(src: &'a str, name: &str) -> &'a str {
		let needle = format!("fn {name}(");
		let start = src.find(&needle).unwrap_or_else(|| panic!("missing fn {name}"));
		let after = &src[start..];
		let mut depth = 0i32;
		let mut end = 0;
		for (i, c) in after.char_indices() {
			match c {
				'{' => depth += 1,
				'}' => {
					depth -= 1;
					if depth == 0 {
						end = i + 1;
						break;
					}
				}
				_ => {}
			}
		}
		assert!(end > 0, "unclosed fn {name}");
		after[..end].trim()
	}

	fn collapse_ws(s: &str) -> String {
		s.split_whitespace().collect::<Vec<_>>().join(" ")
	}

	#[test]
	fn material_shader_keeps_the_shared_hash_bodies() -> anyhow::Result<()> {
		for name in ["hash13", "hash21", "link_weakness"] {
			assert_eq!(
				collapse_ws(wgsl_fn_body(HASH_WGSL, name)),
				collapse_ws(wgsl_fn_body(CHAIN_MATERIAL_WGSL, name)),
				"{name} in chain_material.wgsl drifted from hash.wgsl"
			);
		}
		Ok(())
	}

	#[test]
	fn rust_constants_match_the_wgsl_literals() -> anyhow::Result<()> {
		let scale = parse_vec3_literal(HASH_WGSL, "p * vec3<f32>(").expect("scale");
		assert_eq!(scale, HASH13_SCALE);
		let offset = parse_f32_after(HASH_WGSL, "p3.yzx + vec3<f32>(").expect("offset");
		assert_eq!(offset, HASH13_OFFSET);
		let hash21_z = parse_f32_after(HASH_WGSL, "vec3<f32>(p, ").expect("hash21 z");
		assert_eq!(hash21_z, HASH21_Z);
		assert_eq!(parse_f32_after(HASH_WGSL, "id * ").expect("id"), WEAKNESS_ID_SCALE);
		assert_eq!(parse_f32_after(HASH_WGSL, "seed * ").expect("seed"), WEAKNESS_SEED_SCALE);
		assert_eq!(parse_f32_after(HASH_WGSL, "seed * 0.013, ").expect("z"), WEAKNESS_Z);
		Ok(())
	}

	#[test]
	fn golden_hash13_matches_wgsl_transcription() -> anyhow::Result<()> {
		// Bit-exact f32 results of hash13 on HASH13_GOLDEN_INPUTS.
		let expected = [
			f32::from_bits(0x0000_0000),
			f32::from_bits(0x3e6e_9d80),
			f32::from_bits(0x3ea0_fdc0),
			f32::from_bits(0x3f3d_a220),
		];
		assert_eq!(HASH13_GOLDEN_INPUTS.len(), expected.len());
		for (i, &p) in HASH13_GOLDEN_INPUTS.iter().enumerate() {
			let cpu = hash13(p);
			let from_wgsl = hash13_from_wgsl_source(p);
			assert!((cpu - from_wgsl).abs() < 1e-6, "cpu {cpu} vs wgsl-text {from_wgsl} at {p:?}");
			assert_eq!(
				cpu.to_bits(),
				expected[i].to_bits(),
				"cpu {cpu} vs golden {} at {p:?}",
				expected[i]
			);
		}
		Ok(())
	}

	#[test]
	fn golden_weakness_matches_wgsl_transcription() -> anyhow::Result<()> {
		// Bit-exact f32 results of link_weakness on WEAKNESS_GOLDEN_INPUTS.
		let expected = [
			f32::from_bits(0x3f1e_e220),
			f32::from_bits(0x3ea0_fdc0),
			f32::from_bits(0x3ea5_e6c0),
			f32::from_bits(0x3e14_1e00),
			f32::from_bits(0x3e20_d000),
		];
		assert_eq!(WEAKNESS_GOLDEN_INPUTS.len(), expected.len());
		for (i, &(id, seed)) in WEAKNESS_GOLDEN_INPUTS.iter().enumerate() {
			let cpu = link_weakness(id, seed);
			let from_wgsl = link_weakness_from_wgsl_source(id, seed);
			assert!(
				(cpu - from_wgsl).abs() < 1e-6,
				"cpu {cpu} vs wgsl-text {from_wgsl} for id={id} seed={seed}"
			);
			assert_eq!(
				cpu.to_bits(),
				expected[i].to_bits(),
				"cpu {cpu} vs golden {} for id={id} seed={seed}",
				expected[i]
			);
			assert!((0.0..=1.0).contains(&cpu));
		}
		assert_ne!(link_weakness(3.0, 11.0), link_weakness(4.0, 11.0));
		assert_ne!(link_weakness(3.0, 11.0), link_weakness(3.0, 29.0));
		Ok(())
	}

	#[test]
	fn wgsl_fract_matches_floor_not_trunc() -> anyhow::Result<()> {
		assert!((wgsl_fract(1.25) - 0.25).abs() < 1e-6);
		assert!((wgsl_fract(-1.25) - 0.75).abs() < 1e-6);
		assert!(((-1.25_f32).fract() + 0.25).abs() < 1e-6);
		Ok(())
	}
}
