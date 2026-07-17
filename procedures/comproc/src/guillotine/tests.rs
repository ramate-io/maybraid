use super::*;
use crate::noise::config::NoiseConfig;
use anyhow::{bail, Result};
use bevy::math::{Vec2, Vec3, Vec4};
use noise::{NoiseFn, Perlin, Seedable};

fn cutter<const D: usize>(seed: u32, config: GuillotineConfig, depth: u8) -> Guillotine<D, Perlin>
where
	Perlin: NoiseFn<f64, D>,
{
	let noise = NoiseConfig::new(Perlin::default())
		.with_seed(seed)
		.with_frequency(0.05)
		.with_amplitude(1.0)
		.with_octaves(1);
	Guillotine::new(noise, config, depth)
}

fn assert_tiles_parent<const D: usize>(parent: Bounds<D>, leaves: &[Bounds<D>]) -> Result<()> {
	if leaves.is_empty() {
		bail!("expected at least one leaf");
	}
	let parent_vol = parent.volume();
	let leaf_vol: f32 = leaves.iter().map(Bounds::volume).sum();
	if (leaf_vol - parent_vol).abs() > 1e-3 * parent_vol.max(1.0) {
		bail!("volume mismatch: parent={parent_vol} leaves={leaf_vol}");
	}
	for i in 0..leaves.len() {
		for j in (i + 1)..leaves.len() {
			if interiors_overlap(&leaves[i], &leaves[j]) {
				bail!("leaves {i} and {j} overlap in interior");
			}
		}
	}
	Ok(())
}

fn interiors_overlap<const D: usize>(a: &Bounds<D>, b: &Bounds<D>) -> bool {
	for i in 0..D {
		if a.max[i] <= b.min[i] || b.max[i] <= a.min[i] {
			return false;
		}
	}
	true
}

#[test]
fn depth_zero_returns_root() -> Result<()> {
	let g = cutter::<1>(1, GuillotineConfig::new(1.0, 4.0), 0);
	let root = Bounds1::from_interval(0.0, 10.0);
	let leaves: Vec<_> = g.regions(root).into_iter().collect();
	assert_eq!(leaves.len(), 1);
	assert_eq!(leaves[0], root);
	Ok(())
}

#[test]
fn d1_tiles_interval() -> Result<()> {
	let cfg = GuillotineConfig::new(1.0, 4.0);
	let g = cutter::<1>(42, cfg, 6);
	let root = Bounds1::from_interval(0.0, 16.0);
	let leaves = g.regions_vec(root);
	assert!(leaves.len() > 1);
	assert_tiles_parent(root, &leaves)?;
	Ok(())
}

#[test]
fn d2_tiles_rectangle() -> Result<()> {
	let cfg = GuillotineConfig::new(4.0, 12.0);
	let g = cutter::<2>(7, cfg, 8);
	let root = Bounds2::from_vec2(Vec2::new(10.0, 20.0), Vec2::new(74.0, 84.0));
	let leaves = g.regions_vec(root);
	assert!(leaves.len() > 1);
	assert_tiles_parent(root, &leaves)?;
	Ok(())
}

#[test]
fn d3_tiles_box() -> Result<()> {
	let cfg = GuillotineConfig::new(2.0, 8.0);
	let g = cutter::<3>(99, cfg, 6);
	let root = Bounds3::from_vec3(Vec3::ZERO, Vec3::splat(32.0));
	let leaves = g.regions_vec(root);
	assert!(leaves.len() > 1);
	assert_tiles_parent(root, &leaves)?;
	Ok(())
}

#[test]
fn d4_tiles_hyperbox() -> Result<()> {
	let cfg = GuillotineConfig::new(2.0, 6.0);
	let g = cutter::<4>(3, cfg, 6);
	let root = Bounds4::from_vec4(Vec4::ZERO, Vec4::splat(16.0));
	let leaves = g.regions_vec(root);
	assert!(leaves.len() > 1);
	assert_tiles_parent(root, &leaves)?;
	Ok(())
}

#[test]
fn partition_is_deterministic() -> Result<()> {
	let g = cutter::<2>(1234, GuillotineConfig::default(), 5);
	let root = Bounds2::from_origin_extent([0.0, 0.0], [64.0, 64.0]);
	assert_eq!(g.regions_vec(root), g.regions_vec(root));
	Ok(())
}

#[test]
fn oversized_steps_saturate_without_cuts() -> Result<()> {
	// Steps always exceed half-extent from mid → no interior cuts.
	let cfg = GuillotineConfig::new(12.0, 16.0);
	let g = cutter::<2>(5, cfg, 8);
	let root = Bounds2::from_origin_extent([0.0, 0.0], [10.0, 10.0]);
	let cuts = g.cut(root);
	assert!(cuts.cuts[0].is_empty());
	assert!(cuts.cuts[1].is_empty());
	let leaves = cuts.regions_vec();
	assert_eq!(leaves.len(), 1);
	assert_eq!(leaves[0], root);
	Ok(())
}

#[test]
fn middle_out_places_cuts_on_both_sides_of_mid() -> Result<()> {
	let cfg = GuillotineConfig::new(2.0, 5.0);
	let g = cutter::<1>(17, cfg, 8);
	let root = Bounds1::from_interval(0.0, 40.0);
	let mid = 20.0;
	let cuts = g.cut(root);
	assert!(cuts.cuts[0].len() >= 2);
	assert!(cuts.cuts[0].iter().any(|&c| c < mid));
	assert!(cuts.cuts[0].iter().any(|&c| c > mid));
	assert_tiles_parent(root, &cuts.regions_vec())?;
	Ok(())
}

#[test]
fn snap_quantum_aligns_cuts() -> Result<()> {
	let cfg = GuillotineConfig::new(2.0, 6.0).with_snap_quantum(4.0);
	let g = cutter::<1>(11, cfg, 4);
	let root = Bounds1::from_interval(0.0, 32.0);
	let cuts = g.cut(root);
	assert_tiles_parent(root, &cuts.regions_vec())?;
	for &c in &cuts.cuts[0] {
		let q = (c / 4.0).round() * 4.0;
		if (c - q).abs() > 1e-4 {
			bail!("cut not on quantum: {c}");
		}
	}
	Ok(())
}

#[test]
fn regions_iter_matches_region_count() -> Result<()> {
	let g = cutter::<2>(3, GuillotineConfig::new(4.0, 10.0), 5);
	let root = Bounds2::from_origin_extent([0.0, 0.0], [40.0, 40.0]);
	let cuts = g.cut(root);
	assert_eq!(cuts.regions().count(), cuts.region_count());
	Ok(())
}

#[test]
fn variable_depth_samples_and_tiles() -> Result<()> {
	let noise = NoiseConfig::new(Perlin::default())
		.with_seed(21)
		.with_frequency(0.05)
		.with_amplitude(1.0)
		.with_octaves(1);
	let v = VariableGuillotine::<2, _>::new(
		noise,
		GuillotineConfig::new(3.0, 9.0),
		DepthRange::new(2, 6),
	);
	let root = Bounds2::from_origin_extent([0.0, 0.0], [48.0, 48.0]);
	let leaves = v.regions_vec(root);
	assert!(!leaves.is_empty());
	assert_tiles_parent(root, &leaves)?;
	// Same root → same sampled depth → same partition.
	assert_eq!(leaves, v.regions_vec(root));
	Ok(())
}

#[test]
fn noise_accessors_round_trip() -> Result<()> {
	let mut g = cutter::<2>(1, GuillotineConfig::default(), 3);
	let seed = g.noise().noise.seed();
	g.set_noise(
		NoiseConfig::new(Perlin::default())
			.with_seed(seed.wrapping_add(9))
			.with_frequency(0.1),
	);
	assert_eq!(g.noise().noise.seed(), seed.wrapping_add(9));
	g.noise_mut().frequency = 0.2;
	assert!((g.noise().frequency - 0.2).abs() < 1e-6);
	Ok(())
}
