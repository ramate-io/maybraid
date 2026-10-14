//! Release timing for bounded POI registry queries vs catalog global scans.
//!
//! ```bash
//! CXX=g++ cargo test -p poi-intelligence registry_spatial_timing --release -- --ignored --nocapture
//! ```

use std::hint::black_box;
use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::{NearbyQuery, Poi, PoiId, PoiInterests, PoiKind, PoiRegistry};

const POI_COUNTS: [usize; 4] = [128, 512, 2048, 8192];
const GLOBAL_FRACTION: usize = 8;
const NEARBY_RADIUS: f32 = 160.0;
const TILE_SIZE: f32 = 256.0;
const PROBES: usize = 64;
const WARMUP: usize = 8;
const WARM_ITERS: usize = 32;
const COLD_ITERS: usize = 4;

const KIND: PoiKind = PoiKind::new("bench/camp");
const RERUN: &str =
	"CXX=g++ cargo test -p poi-intelligence registry_spatial_timing --release -- --ignored --nocapture";

#[derive(Clone, Copy, Debug)]
struct Stats {
	median: Duration,
	p95: Duration,
	total: Duration,
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
	if sorted.is_empty() {
		return Duration::ZERO;
	}
	let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
	sorted[idx.min(sorted.len() - 1)]
}

fn stats(mut samples: Vec<Duration>) -> Stats {
	samples.sort_unstable();
	Stats {
		median: percentile(&samples, 0.50),
		p95: percentile(&samples, 0.95),
		total: samples.iter().copied().sum(),
	}
}

fn us(d: Duration) -> f64 {
	d.as_secs_f64() * 1_000_000.0
}

fn fmt_us(d: Duration) -> String {
	format!("{:.2}", us(d))
}

fn fmt_total(d: Duration) -> String {
	let ms = d.as_secs_f64() * 1_000.0;
	if ms >= 1_000.0 {
		format!("{:.2} s", d.as_secs_f64())
	} else {
		format!("{:.2} ms", ms)
	}
}

fn seeded_registry(count: usize) -> (PoiRegistry, Vec<Vec3>, Vec<IVec2>) {
	let mut registry = PoiRegistry::default();
	let interests = PoiInterests::one(KIND);
	let width = (count as f64).sqrt().ceil() as f32;
	let mut centers = Vec::with_capacity(PROBES);
	let mut tiles = Vec::with_capacity(PROBES);
	for index in 0..count {
		let id = PoiId(index as u64 + 1);
		let gx = (index as f32 % width) * 48.0 - width * 24.0;
		let gz = (index as f32 / width).floor() * 48.0 - width * 24.0;
		let position = Vec3::new(gx, 0.0, gz);
		let global = index % GLOBAL_FRACTION == 0;
		let local = index % 3 != 0 || !global;
		registry
			.upsert(
				Entity::from_bits(index as u64 + 1),
				Poi::new(id, KIND).with_arrival_radius(4.0),
				position,
				local,
				global,
			)
			.expect("upsert");
		if index < PROBES {
			centers.push(position);
			tiles.push((position.xz() / TILE_SIZE).floor().as_ivec2());
		}
	}
	black_box(&interests);
	(registry, centers, tiles)
}

/// Pre-change `collect_nearby_in` global catalog scan (local spatial + every global).
fn naive_collect_nearby_in(
	registry: &PoiRegistry,
	center: Vec3,
	query: NearbyQuery,
	interests: &PoiInterests,
	out: &mut Vec<PoiId>,
) {
	out.clear();
	if interests.is_empty() || !center.is_finite() || !query.radius.is_finite() {
		return;
	}
	let radius = query.radius.clamp(0.0, 1_000.0);
	let min_radius = query.min_radius.max(0.0);
	registry.collect_local_matching(center, radius, interests, out);
	for id in registry.global_matching(interests).iter().map(|record| record.id) {
		if out.contains(&id) {
			continue;
		}
		let Some(record) = registry.get(id) else {
			continue;
		};
		if center.distance(record.position) <= radius + record.arrival_radius {
			out.push(id);
		}
	}
	out.retain(|id| {
		registry
			.get(*id)
			.is_some_and(|record| (center.xz() - record.position.xz()).length() >= min_radius)
	});
	out.sort_unstable_by_key(|id| *id);
}

/// Pre-change xz-tile path: spatial query over locals only, then `O(globals)` scan.
fn naive_matching_in_xz_tile(
	registry: &PoiRegistry,
	tile: IVec2,
	tile_size: f32,
	center_y: f32,
	interests: &PoiInterests,
) -> Vec<PoiId> {
	let mut ids: Vec<PoiId> = registry
		.matching_in_xz_tile(tile, tile_size, center_y, interests)
		.into_iter()
		.filter(|record| record.local)
		.map(|record| record.id)
		.collect();
	for record in registry.global_matching(interests) {
		if (record.position.xz() / tile_size).floor().as_ivec2() == tile
			&& !ids.contains(&record.id)
		{
			ids.push(record.id);
		}
	}
	ids.sort_unstable();
	ids
}

fn bench_nearby(registry: &PoiRegistry, centers: &[Vec3], interests: &PoiInterests) -> (Stats, Stats) {
	let query = NearbyQuery::weighted(NEARBY_RADIUS);
	let mut scratch = Vec::new();
	let mut warm_naive = Vec::new();
	let mut warm_spatial = Vec::new();
	for _ in 0..WARMUP {
		for center in centers {
			naive_collect_nearby_in(registry, *center, query, interests, &mut scratch);
			registry.collect_nearby_in(*center, query, interests, &[], &mut scratch);
		}
	}
	for _ in 0..WARM_ITERS {
		for center in centers {
			let start = Instant::now();
			naive_collect_nearby_in(registry, *center, query, interests, &mut scratch);
			warm_naive.push(start.elapsed());
			let start = Instant::now();
			registry.collect_nearby_in(*center, query, interests, &[], &mut scratch);
			warm_spatial.push(start.elapsed());
		}
	}
	let mut cold_naive = Vec::new();
	let mut cold_spatial = Vec::new();
	for _ in 0..COLD_ITERS {
		for center in centers {
			let start = Instant::now();
			naive_collect_nearby_in(registry, *center, query, interests, &mut scratch);
			cold_naive.push(start.elapsed());
			let start = Instant::now();
			registry.collect_nearby_in(*center, query, interests, &[], &mut scratch);
			cold_spatial.push(start.elapsed());
		}
	}
	(bench_stats(warm_naive, cold_naive), bench_stats(warm_spatial, cold_spatial))
}

fn bench_tile(registry: &PoiRegistry, tiles: &[IVec2], interests: &PoiInterests) -> (Stats, Stats) {
	let mut warm_naive = Vec::new();
	let mut warm_spatial = Vec::new();
	for _ in 0..WARMUP {
		for tile in tiles {
			black_box(naive_matching_in_xz_tile(registry, *tile, TILE_SIZE, 0.0, interests));
			black_box(registry.matching_in_xz_tile(*tile, TILE_SIZE, 0.0, interests));
		}
	}
	for _ in 0..WARM_ITERS {
		for tile in tiles {
			let start = Instant::now();
			black_box(naive_matching_in_xz_tile(registry, *tile, TILE_SIZE, 0.0, interests));
			warm_naive.push(start.elapsed());
			let start = Instant::now();
			black_box(registry.matching_in_xz_tile(*tile, TILE_SIZE, 0.0, interests));
			warm_spatial.push(start.elapsed());
		}
	}
	let mut cold_naive = Vec::new();
	let mut cold_spatial = Vec::new();
	for _ in 0..COLD_ITERS {
		for tile in tiles {
			let start = Instant::now();
			black_box(naive_matching_in_xz_tile(registry, *tile, TILE_SIZE, 0.0, interests));
			cold_naive.push(start.elapsed());
			let start = Instant::now();
			black_box(registry.matching_in_xz_tile(*tile, TILE_SIZE, 0.0, interests));
			cold_spatial.push(start.elapsed());
		}
	}
	(bench_stats(warm_naive, cold_naive), bench_stats(warm_spatial, cold_spatial))
}

fn bench_stats(warm: Vec<Duration>, cold: Vec<Duration>) -> Stats {
	let _ = cold;
	stats(warm)
}

#[test]
#[ignore]
fn registry_spatial_timing() {
	let interests = PoiInterests::one(KIND);
	eprintln!("registry spatial timing ({RERUN})\n");
	eprintln!(
		"| n | op | path | warm med µs | warm p95 µs | warm total | naive med µs | naive p95 µs |"
	);
	eprintln!("| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: |");
	for count in POI_COUNTS {
		let (registry, centers, tiles) = seeded_registry(count);
		let (naive_nearby, spatial_nearby) = bench_nearby(&registry, &centers, &interests);
		eprintln!(
			"| {count} | nearby | spatial | {sm} | {sp} | {st} | {nm} | {np} |",
			sm = fmt_us(spatial_nearby.median),
			sp = fmt_us(spatial_nearby.p95),
			st = fmt_total(spatial_nearby.total),
			nm = fmt_us(naive_nearby.median),
			np = fmt_us(naive_nearby.p95),
		);
		let (naive_tile, spatial_tile) = bench_tile(&registry, &tiles, &interests);
		eprintln!(
			"| {count} | xz_tile | spatial | {sm} | {sp} | {st} | {nm} | {np} |",
			sm = fmt_us(spatial_tile.median),
			sp = fmt_us(spatial_tile.p95),
			st = fmt_total(spatial_tile.total),
			nm = fmt_us(naive_tile.median),
			np = fmt_us(naive_tile.p95),
		);
	}
}
