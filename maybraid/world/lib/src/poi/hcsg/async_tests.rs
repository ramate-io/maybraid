use std::time::{Duration, Instant};

use super::*;
use lod::hcsg::{spawn_worker, HcsgClass, MAX_TICKET_REQUESTS};
use poi_intelligence::{PoiId, PoiInterest};

struct Pins<const N: u64>;

impl<const N: u64> PoiOrigin for Pins<N> {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::from_cell(region))]
	}
	fn pois(_: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let mut position = Vec3::from((bounds.min + bounds.max) * 0.5);
		position.y = N as f32;
		Some((
			vec![PoiEntry {
				id: PoiId(N),
				kind: URBAN_POI,
				position,
				arrival_radius: 7.0,
				support: PoiSupport::Floor,
				salience: 1.7,
				local: true,
				global: false,
			}],
			bounds,
		))
	}
}

fn query(x: i32) -> PoiTileQuery {
	PoiTileQuery {
		tile: IVec2::new(x, 0),
		tile_size: 256.0,
		center_y: 0.0,
		interests: PoiInterests::new([PoiInterest::new(URBAN_POI, 1.0)]),
	}
}

fn group(query: PoiTileQuery) -> WorldPoiRequest {
	WorldPoiRequest {
		families: vec![
			Box::new(PoiTileRequest::<Pins<1>>::new(query.clone())),
			Box::new(PoiTileRequest::<Pins<2>>::new(query.clone())),
			Box::new(PoiTileRequest::<Pins<3>>::new(query)),
		],
	}
}

#[test]
fn capacity_rolls_back_partial_family_admission() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut fillers = Vec::new();
	for x in 0..MAX_TICKET_REQUESTS - 1 {
		let region = poi_intelligence::xz_tile_region(IVec2::new(x as i32, 10), 256.0, 0.0)
			.ok_or_else(|| anyhow::anyhow!("region"))?;
		fillers.push(
			demand
				.try_request::<PoiSet<Pins<4>>>(region, HcsgClass::Near)
				.map_err(|e| anyhow::anyhow!("request: {e:?}"))?,
		);
	}
	let mut request = group(query(0));
	let mut out = Vec::new();
	assert_eq!(
		request
			.poll(&storage, &demand, &mut out, true)
			.map_err(|_| anyhow::anyhow!("busy"))?,
		PoiTilePoll::Capacity
	);
	assert!(out.is_empty());
	// This slot was briefly held by family 1. Rollback must release it even
	// while families 2/3 never acquired tickets, allowing unrelated demand in.
	let spare = demand
		.try_request::<PoiSet<Pins<4>>>(
			poi_intelligence::xz_tile_region(IVec2::new(0, 20), 256.0, 0.0)
				.ok_or_else(|| anyhow::anyhow!("region"))?,
			HcsgClass::Near,
		)
		.map_err(|e| anyhow::anyhow!("rollback left slot pinned: {e:?}"))?;
	drop(spare);
	drop(fillers);
	let _worker = spawn_worker(storage.clone(), demand.clone())?;
	let deadline = Instant::now() + Duration::from_secs(5);
	loop {
		match request.poll(&storage, &demand, &mut out, true) {
			Ok(PoiTilePoll::Ready) => break,
			_ => {
				anyhow::ensure!(Instant::now() < deadline, "request did not recover after rollback")
			}
		}
		assert!(demand.wait_idle(Duration::from_secs(5)));
	}
	assert_eq!(out.iter().map(|p| p.id).collect::<Vec<_>>(), [PoiId(1), PoiId(2), PoiId(3)]);
	assert!(out
		.iter()
		.all(|p| p.support == PoiSupport::Floor && p.arrival_radius == 7.0 && p.salience == 1.7));
	Ok(())
}

#[test]
fn more_multifamily_consumers_than_pool_slots_all_complete() -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let _worker = spawn_worker(storage.clone(), demand.clone())?;
	let mut requests: Vec<_> =
		(0..MAX_TICKET_REQUESTS + 5).map(|x| Some(group(query(x as i32)))).collect();
	let deadline = Instant::now() + Duration::from_secs(10);
	let mut complete = 0;
	while complete < requests.len() {
		for slot in &mut requests {
			let Some(request) = slot else {
				continue;
			};
			let mut out = Vec::new();
			match request.poll(&storage, &demand, &mut out, true) {
				Ok(PoiTilePoll::Ready) => {
					assert_eq!(out.len(), 3);
					*slot = None;
					complete += 1;
				}
				Ok(PoiTilePoll::Stale) => request.release(),
				_ => {}
			}
		}
		anyhow::ensure!(
			Instant::now() < deadline,
			"multi-family requests deadlocked: {complete} completed"
		);
		std::thread::sleep(Duration::from_millis(1));
	}
	Ok(())
}

struct AvailableFamily {
	busy: bool,
}
impl DistantTileRequest for AvailableFamily {
	fn poll(
		&mut self,
		_: &HcsgStorage,
		_: &HcsgDemand,
		out: &mut Vec<PoiEntry>,
		_: bool,
	) -> Result<PoiTilePoll, Busy> {
		out.clear();
		if self.busy {
			return Err(Busy);
		}
		let position = Vec3::new(1.0, 2.0, 3.0);
		out.push(PoiEntry {
			id: PoiId(7),
			kind: URBAN_POI,
			position,
			arrival_radius: 1.0,
			support: PoiSupport::Terrain,
			salience: 1.0,
			local: false,
			global: true,
		});
		Ok(PoiTilePoll::Pending)
	}
}

#[test]
fn world_exposes_available_family_without_waiting_for_pending_or_busy_families(
) -> anyhow::Result<()> {
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let mut request = WorldPoiRequest {
		families: vec![
			Box::new(AvailableFamily { busy: false }),
			Box::new(AvailableFamily { busy: true }),
		],
	};
	let mut out = Vec::new();
	for _ in 0..2 {
		assert_eq!(
			request
				.poll(&storage, &demand, &mut out, true)
				.map_err(|_| anyhow::anyhow!("busy"))?,
			PoiTilePoll::Pending
		);
		assert_eq!(out.iter().map(|poi| poi.id).collect::<Vec<_>>(), [PoiId(7)]);
	}
	Ok(())
}

struct OrderedPins;
impl PoiOrigin for OrderedPins {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		vec![OriginalId(Id::from_cell(region))]
	}
	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let (mut out, bounds) = Pins::<9>::pois(cx, id)?;
		out.extend(Pins::<2>::pois(cx, id)?.0);
		out.extend(Pins::<9>::pois(cx, id)?.0);
		Some((out, bounds))
	}
}

#[test]
fn cached_pois_are_available_before_discovery_and_when_admission_is_saturated() -> anyhow::Result<()>
{
	let storage = HcsgStorage::default();
	let demand = HcsgDemand::default();
	let query = query(0);
	let region = poi_intelligence::xz_tile_region(query.tile, query.tile_size, query.center_y)
		.ok_or_else(|| anyhow::anyhow!("region"))?;
	let mut cx = GenerationContext::new(&storage);
	cx.get_or_generate::<PoiSet<OrderedPins>>(Id::from_cell(region))
		.ok_or_else(|| anyhow::anyhow!("cached pins"))?;
	let mut request = PoiTileRequest::<OrderedPins>::new(query.clone());
	let mut out = Vec::new();
	assert_eq!(
		request
			.poll(&storage, &demand, &mut out, true)
			.map_err(|_| anyhow::anyhow!("busy"))?,
		PoiTilePoll::Pending
	);
	assert_eq!(out.iter().map(|poi| poi.id).collect::<Vec<_>>(), [PoiId(2), PoiId(9)]);
	assert_eq!(
		demand
			.try_outstanding(&[HcsgClass::Near])
			.map_err(|_| anyhow::anyhow!("busy"))?
			.undiscovered,
		1,
		"reading available data keeps generation demand live"
	);
	request.release();
	let mut fillers = Vec::new();
	for x in 0..MAX_TICKET_REQUESTS {
		let region = poi_intelligence::xz_tile_region(IVec2::new(x as i32, 10), 256.0, 0.0)
			.ok_or_else(|| anyhow::anyhow!("filler"))?;
		fillers.push(
			demand
				.try_request::<PoiSet<Pins<4>>>(region, HcsgClass::Near)
				.map_err(|error| anyhow::anyhow!("request: {error:?}"))?,
		);
	}
	assert_eq!(
		request
			.poll(&storage, &demand, &mut out, true)
			.map_err(|_| anyhow::anyhow!("busy"))?,
		PoiTilePoll::Capacity
	);
	assert_eq!(out.iter().map(|poi| poi.id).collect::<Vec<_>>(), [PoiId(2), PoiId(9)]);
	Ok(())
}
