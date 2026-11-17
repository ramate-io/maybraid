use std::sync::atomic::Ordering;

use bevy::math::Vec3;
use bevy::shape::Aabb3d;

use crate::gen::{Id, OriginalId};
use crate::hcsg::demand::{HcsgDemand, QuantumProgress, PUBLICATION_PAGE_IDS};
use crate::hcsg::request::HcsgRequest;
use crate::hcsg::{GenerationContext, GenerationScheme, HcsgClass, HcsgRegions};

struct Probe;
impl GenerationScheme for Probe {
	fn original_ids_for(_: &mut GenerationContext, _: Aabb3d) -> Vec<OriginalId> {
		Vec::new()
	}
	fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		None
	}
}

fn span(x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
}
fn wanted(request: &mut HcsgRequest, regions: Vec<Aabb3d>) {
	request.apply_regions(&HcsgRegions::<()>::new(regions, None, HcsgClass::Near));
}
fn sync(request: &mut HcsgRequest, demand: &HcsgDemand) -> anyhow::Result<()> {
	request.sync::<Probe>(demand).map_err(|_| anyhow::anyhow!("demand busy"))?;
	Ok(())
}

#[test]
fn frame_request_contention_preserves_all_request_and_registry_state() -> anyhow::Result<()> {
	let demand = HcsgDemand::default();
	let mut request = HcsgRequest::default();
	wanted(&mut request, vec![span(0.0)]);
	let before = format!("{request:?}");
	let guard = demand.lock();
	assert!(request.sync::<Probe>(&demand).is_err());
	assert_eq!(format!("{request:?}"), before);
	assert!(guard.subscriptions.is_empty());
	drop(guard);
	sync(&mut request, &demand)?;
	let first = request.subscription().ok_or_else(|| anyhow::anyhow!("subscription"))?;
	let (reach, cancelled) = {
		let mut state = demand.lock();
		let old = state.subscriptions.get_mut(&first).ok_or_else(|| anyhow::anyhow!("old"))?;
		old.pass = 17.0;
		(old.reach.clone(), old.cancelled.clone())
	};
	let reach_guard = reach.lock().map_err(|_| anyhow::anyhow!("reach poisoned"))?;
	wanted(&mut request, vec![span(2.0)]);
	let before = format!("{request:?}");
	let next_id = demand.0.next_id.load(Ordering::Relaxed);
	assert!(request.sync::<Probe>(&demand).is_err(), "nested reach must use try-lock too");
	assert_eq!(format!("{request:?}"), before);
	assert!(!cancelled.load(Ordering::Acquire));
	assert_eq!(demand.0.next_id.load(Ordering::Relaxed), next_id);
	assert_eq!(demand.lock().subscriptions.len(), 1);
	drop(reach_guard);
	sync(&mut request, &demand)?;
	let second = request.subscription().ok_or_else(|| anyhow::anyhow!("replacement"))?;
	assert_ne!(first, second);
	assert!(cancelled.load(Ordering::Acquire));
	assert_eq!(demand.pass_of(second), Some(17.0));
	wanted(&mut request, Vec::new());
	let before = format!("{request:?}");
	let guard = demand.lock();
	assert!(request.sync::<Probe>(&demand).is_err());
	assert_eq!(format!("{request:?}"), before);
	assert!(guard.subscriptions.contains_key(&second));
	drop(guard);
	sync(&mut request, &demand)?;
	assert!(request.subscription().is_none());
	assert!(demand.lock().subscriptions.is_empty());
	Ok(())
}

#[test]
fn frame_transaction_detects_epoch_drop_and_reuses_shared_reach() -> anyhow::Result<()> {
	let demand = HcsgDemand::default();
	let mut request = HcsgRequest::default();
	wanted(&mut request, vec![span(0.0)]);
	sync(&mut request, &demand)?;
	let old = request.subscription().ok_or_else(|| anyhow::anyhow!("old"))?;
	let reach = demand
		.lock()
		.subscriptions
		.get(&old)
		.ok_or_else(|| anyhow::anyhow!("old subscription"))?
		.reach
		.clone();
	wanted(&mut request, vec![span(2.0)]);
	sync(&mut request, &demand)?;
	let replacement = request.subscription().ok_or_else(|| anyhow::anyhow!("replacement"))?;
	let current_reach = demand
		.lock()
		.subscriptions
		.get(&replacement)
		.ok_or_else(|| anyhow::anyhow!("replacement subscription"))?
		.reach
		.clone();
	assert!(std::sync::Arc::ptr_eq(&reach, &current_reach));
	demand.advance_epoch();
	let result = request
		.sync::<Probe>(&demand)
		.map_err(|_| anyhow::anyhow!("busy"))?
		.ok_or_else(|| anyhow::anyhow!("epoch must resubscribe"))?;
	assert!(result.epoch_advanced);
	assert_ne!(request.subscription(), Some(replacement));
	Ok(())
}

#[test]
fn publication_pages_bound_completed_backlogs_and_preserve_metadata() -> anyhow::Result<()> {
	let demand = HcsgDemand::default();
	let id = demand.subscribe::<Probe>(None, vec![span(0.0)], None, HcsgClass::Near);
	let ids: Vec<_> = (0..70).map(|x| Id::from_cell(span(x as f32))).collect();
	demand.finish_quantum(
		id,
		QuantumProgress { done: true, published: ids.clone(), ..Default::default() },
		None,
	);
	let mut read = Vec::new();
	for expected in [false, false, true] {
		let page = demand
			.try_read(id, read.len())
			.map_err(|_| anyhow::anyhow!("busy"))?
			.ok_or_else(|| anyhow::anyhow!("page"))?;
		assert!(page.ids.len() <= PUBLICATION_PAGE_IDS);
		assert_eq!(page.done, expected);
		assert!(!page.failed);
		assert_eq!(page.epoch, 0);
		read.extend(page.ids);
	}
	assert_eq!(read, ids);
	assert!(demand.try_page_is_live(id, 0).map_err(|_| anyhow::anyhow!("busy"))?);
	demand.advance_epoch();
	assert!(!demand.try_page_is_live(id, 0).map_err(|_| anyhow::anyhow!("busy"))?);
	assert!(demand.try_read(id, 0).map_err(|_| anyhow::anyhow!("busy"))?.is_none());
	Ok(())
}
