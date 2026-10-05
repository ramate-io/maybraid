//! Release micro-benchmark for exclude-buffer construction.
//!
//! Run:
//! ```bash
//! nix develop --command cargo test -p ground-avian --release column_skip_buffer_timing -- --ignored --nocapture
//! ```

use std::hint::black_box;
use std::time::Instant;

use bevy::ecs::entity::Entity;
use ground_avian::column_skip::ColumnSkipBuffer;

const ITERATIONS: usize = 200_000;

fn sample_entities(count: usize) -> Vec<Entity> {
	(0..count)
		.map(|index| Entity::from_raw_u32(index as u32 + 1).expect("test entity"))
		.collect()
}

fn legacy_skip_buffer(exclude: &[Entity], column_hits: &[Entity]) -> Vec<Entity> {
	let mut skipped = exclude.to_vec();
	for entity in column_hits {
		skipped.push(*entity);
	}
	skipped
}

fn stack_skip_buffer(exclude: &[Entity], column_hits: &[Entity]) -> ColumnSkipBuffer {
	let mut skipped = ColumnSkipBuffer::from_exclude(exclude);
	for entity in column_hits {
		skipped.push(*entity);
	}
	skipped
}

fn median_nanos(samples: &mut [u128]) -> f64 {
	samples.sort_unstable();
	let mid = samples.len() / 2;
	if samples.len().is_multiple_of(2) {
		(samples[mid - 1] + samples[mid]) as f64 / 2.0
	} else {
		samples[mid] as f64
	}
}

fn bench<F>(mut f: F) -> f64
where
	F: FnMut(),
{
	let mut samples = [0u128; 5];
	for sample in &mut samples {
		let start = Instant::now();
		for _ in 0..ITERATIONS {
			f();
		}
		*sample = start.elapsed().as_nanos();
	}
	median_nanos(&mut samples) / ITERATIONS as f64
}

#[test]
#[ignore = "release micro-benchmark; run with --release --ignored --nocapture"]
fn column_skip_buffer_timing() {
	let exclude = sample_entities(4);
	let column_hits = sample_entities(3);

	let legacy_ns = bench(|| {
		let skipped = legacy_skip_buffer(black_box(&exclude), black_box(&column_hits));
		black_box(skipped.len());
	});
	let stack_ns = bench(|| {
		let skipped = stack_skip_buffer(black_box(&exclude), black_box(&column_hits));
		black_box(skipped.len());
	});

	eprintln!(
		"column skip buffer ({} iterations, 4 exclude + 3 hits): legacy {:.1} ns/iter, stack {:.1} ns/iter",
		ITERATIONS,
		legacy_ns,
		stack_ns
	);
}
