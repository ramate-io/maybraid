use super::*;
use std::cell::Cell;
thread_local! { static PAINTS: Cell<usize> = const { Cell::new(0) }; }
pub(super) fn record_paint() {
	PAINTS.with(|count| count.set(count.get() + 1));
}

#[test]
fn final_shell_sampler_runs_once_and_usage_once_per_storey() -> anyhow::Result<()> {
	let confines = Confines::new(
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(72.0, 20.0, 72.0)),
		0.0,
		Openings::new(),
	);
	let noise = NoiseParams { seed: 42, ..NoiseParams::default() };
	let sampled = Cell::new(0);
	PAINTS.with(|count| count.set(0));
	let (tower, _) = MixedUseLesHallesMonotower::fit_sampled_with_ceiling(
		&confines,
		noise,
		|shell, noise| {
			sampled.set(sampled.get() + 1);
			let mut params = LesHallesParameterized::sample_monotower(shell, noise)?;
			params.opening_density = 0.35;
			Ok(params)
		},
		RectRingFloorSlab::Solid,
	)?;
	anyhow::ensure!(sampled.get() == 1);
	anyhow::ensure!(PAINTS.with(Cell::get) == tower.floors.len());
	let last = tower
		.floors
		.len()
		.checked_sub(1)
		.ok_or_else(|| anyhow::anyhow!("empty stack"))?;
	for (i, floor) in tower.floors.iter().enumerate() {
		anyhow::ensure!(floor.floor_plan().parameterized.opening_density == 0.35);
		anyhow::ensure!(floor.floor_plan().gallery.has_ceiling() == (i == last));
	}
	let (standalone, _) = MixedUseLesHallesMonotower::fit_to_confines(&confines, noise)?;
	anyhow::ensure!(standalone
		.floors
		.iter()
		.all(|floor| !floor.floor_plan().gallery.has_ceiling()));
	Ok(())
}
