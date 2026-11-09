//! Headless trace/export and overhead fixture. Does not create a window or game.
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::shape::Aabb3d;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{
	spawn_worker, universal_bounds, GenerationContext, GenerationScheme, HcsgClass, HcsgDemand,
	HcsgStorage,
};
use std::time::{Duration, Instant};

struct Dependency(u64);
struct Fixture(u64);
impl GenerationScheme for Dependency {
	const INDEX_SCALE: DVec3 = DVec3::ONE;
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::Universal)]
	}
	fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		Some((Self(41), universal_bounds()))
	}
}
impl GenerationScheme for Fixture {
	const INDEX_SCALE: DVec3 = DVec3::ONE;
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::Universal)]
	}
	fn build_with_id(cx: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		Some((Self(cx.get_or_generate::<Dependency>(Id::Universal)?.0 + 1), universal_bounds()))
	}
}
fn main() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, bevy::log::LogPlugin::default()));
	app.finish();
	std::thread::sleep(Duration::from_secs(2));
	let storage = HcsgStorage::default();
	{
		let mut first = GenerationContext::with_origin(&storage, "fixture.sync");
		let mut second = GenerationContext::with_origin(&storage, "fixture.second");
		let value = first
			.get_or_generate::<Fixture>(Id::Universal)
			.ok_or_else(|| anyhow::anyhow!("fixture missing"))?;
		assert_eq!(value.0, 42);
		assert!(first.get_or_generate::<Fixture>(Id::Universal).is_some());
		drop(first);
		assert!(second.get_or_generate::<Fixture>(Id::Universal).is_some());
		drop(second);
	}
	let demand = HcsgDemand::default();
	let old = demand.subscribe::<Fixture>(None, vec![universal_bounds()], None, HcsgClass::Near);
	demand.subscribe::<Fixture>(Some(old), vec![universal_bounds()], None, HcsgClass::Near);
	std::thread::sleep(Duration::from_millis(10));
	let worker = spawn_worker(storage.clone(), demand.clone())?;
	assert!(demand.wait_idle(Duration::from_secs(10)));
	let started = Instant::now();
	for _ in 0..1000 {
		let mut cx = GenerationContext::with_origin(&storage, "fixture.cache-hit");
		assert!(cx.get_or_generate::<Fixture>(Id::Universal).is_some());
	}
	println!("fixture cache-hit operations=1000 wall_ns={}", started.elapsed().as_nanos());
	drop(worker);
	std::thread::sleep(Duration::from_secs(2));
	Ok(())
}
