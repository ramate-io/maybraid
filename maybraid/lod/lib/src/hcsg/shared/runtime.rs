//! App wiring shared by every generation and presentation plugin.

use bevy::log::error;
use bevy::prelude::{App, IntoScheduleConfigs, Last, SystemSet, Update};

use crate::scene::LodRefreshSystems;

use super::demand::HcsgDemand;
use super::presentation::despawn_retired_hosts;
use super::storage::HcsgStorage;
use super::worker::HcsgWorker;

/// Generation requests and presentation host reconciliation. Runs before the
/// LOD refresh chain, so scene systems see this frame's hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct HcsgSystems;

/// Storage, demand and one worker, once per app.
pub(super) fn ensure_runtime(app: &mut App) {
	if app.world().contains_resource::<HcsgWorker>() {
		return;
	}
	app.configure_sets(Update, HcsgSystems.before(LodRefreshSystems::Track))
		.add_systems(Last, despawn_retired_hosts);
	let storage = app.world_mut().get_resource_or_init::<HcsgStorage>().clone();
	let demand = app.world_mut().get_resource_or_init::<HcsgDemand>().clone();
	match HcsgWorker::spawn(storage, demand) {
		Ok(worker) => {
			app.insert_resource(worker);
		}
		Err(err) => error!("hcsg: failed to spawn the generation worker: {err}"),
	}
}
