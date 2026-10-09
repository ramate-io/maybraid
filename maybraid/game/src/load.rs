//! First-load unveil: Discovery waits on spawn terrain, Near HCSG work, and
//! quiet pending-root fulfill.

use crate::flow::GameFlow;
use crate::shell::ShellRoute;
use bevy::prelude::*;
use maybraid_world::{HcsgClass, HcsgDemand, LodJobCounter, Outstanding, WorldSurfaceReady};
use menu_screens::{request_loading_explainer, request_loading_progress};

/// Remaining Near HCSG ids plus pending-root tickets that still count as
/// "the first wave is finishing." Far, background, and ambient never hold
/// the gate. Streaming continues after unveil.
pub const UNVEIL_JOB_THRESHOLD: u64 = 16;
/// Frames the counter must stay at or below [`UNVEIL_JOB_THRESHOLD`] after
/// work has been observed.
pub const UNVEIL_QUIET_FRAMES: u32 = 2;
/// Allow produce to enqueue before treating a still-zero counter as idle.
pub const UNVEIL_ARM_GRACE_SECS: f32 = 1.0;
/// Safety: spawn collider is ready but jobs never quieted.
pub const UNVEIL_TIMEOUT_SECS: f32 = 25.0;

/// Armed on [`GameFlow::LoadingWorld`]. Prevents unveiling on frame 0 when
/// the job counter is still empty because produce has not run.
#[derive(Resource, Debug, Clone)]
pub struct FirstLoadGate {
	pub saw_work: bool,
	pub quiet_frames: u32,
	pub peak: u64,
	pub entered_at: f32,
}

impl FirstLoadGate {
	pub fn new(entered_at: f32) -> Self {
		Self { saw_work: false, quiet_frames: 0, peak: 0, entered_at }
	}

	pub fn observe(&mut self, active: u64) {
		if active > self.peak {
			self.peak = active;
		}
		if active > UNVEIL_JOB_THRESHOLD {
			self.saw_work = true;
			self.quiet_frames = 0;
			return;
		}
		if self.saw_work {
			self.quiet_frames = self.quiet_frames.saturating_add(1);
		}
	}

	pub fn should_unveil(
		&self,
		ready: bool,
		active: u64,
		near_undiscovered: bool,
		now: f32,
	) -> bool {
		if !ready {
			return false;
		}
		let waited = now - self.entered_at;
		if waited >= UNVEIL_TIMEOUT_SECS {
			return true;
		}
		if near_undiscovered {
			return false;
		}
		if !self.saw_work {
			return waited >= UNVEIL_ARM_GRACE_SECS && active <= UNVEIL_JOB_THRESHOLD;
		}
		active <= UNVEIL_JOB_THRESHOLD && self.quiet_frames >= UNVEIL_QUIET_FRAMES
	}

	pub fn progress(&self, ready: bool, active: u64) -> f32 {
		let from_jobs = if self.peak == 0 {
			if self.saw_work {
				0.55
			} else {
				0.08
			}
		} else {
			(1.0 - (active as f32 / self.peak as f32)).clamp(0.08, 0.95)
		};
		if ready && self.saw_work && active <= UNVEIL_JOB_THRESHOLD {
			return from_jobs.max(0.9);
		}
		if ready {
			from_jobs.max(0.45)
		} else {
			from_jobs.min(0.4)
		}
	}

	pub fn explainer(&self, ready: bool, active: u64) -> &'static str {
		if !ready {
			return "Waiting for the ground…";
		}
		if self.saw_work && active > UNVEIL_JOB_THRESHOLD {
			return "Streaming the world…";
		}
		"Almost ready…"
	}
}

pub(crate) fn arm_first_load(mut commands: Commands, time: Res<Time>) {
	commands.insert_resource(FirstLoadGate::new(time.elapsed_secs()));
}

pub(crate) fn disarm_first_load(mut commands: Commands) {
	commands.remove_resource::<FirstLoadGate>();
}

fn near_outstanding(demand: Option<&HcsgDemand>) -> Outstanding {
	let Some(demand) = demand else {
		return Outstanding::default();
	};
	demand
		.try_outstanding(&[HcsgClass::Near])
		.unwrap_or(Outstanding { undiscovered: 1, remaining: 0 })
}

pub(crate) fn finish_world_loading(
	mut commands: Commands,
	ready: Res<WorldSurfaceReady>,
	jobs: Option<Res<LodJobCounter>>,
	demand: Option<Res<HcsgDemand>>,
	mut gate: Option<ResMut<FirstLoadGate>>,
	time: Res<Time>,
	mut route: ShellRoute,
) {
	let surface_ready = ready.0;
	let near = near_outstanding(demand.as_deref());
	let jobs = jobs.as_deref().map(LodJobCounter::active).unwrap_or(0);
	let active = jobs.saturating_add(near.remaining);
	let undiscovered = near.undiscovered > 0;
	let Some(gate) = gate.as_deref_mut() else {
		if surface_ready && !undiscovered && active <= UNVEIL_JOB_THRESHOLD {
			route.enter(GameFlow::World);
		}
		return;
	};
	gate.observe(active);
	let now = time.elapsed_secs();
	request_loading_progress(&mut commands, gate.progress(surface_ready, active));
	request_loading_explainer(&mut commands, gate.explainer(surface_ready, active));
	if gate.should_unveil(surface_ready, active, undiscovered, now) {
		route.enter(GameFlow::World);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	fn gate_at(entered_at: f32) -> FirstLoadGate {
		FirstLoadGate::new(entered_at)
	}

	#[test]
	fn idle_zero_does_not_unveil_before_grace() {
		let gate = gate_at(0.0);
		assert!(!gate.should_unveil(true, 0, false, 0.2));
		assert!(!gate.should_unveil(true, 0, false, UNVEIL_ARM_GRACE_SECS - 0.01));
	}

	#[test]
	fn idle_after_grace_unveils_when_ready() {
		let gate = gate_at(0.0);
		assert!(gate.should_unveil(true, 0, false, UNVEIL_ARM_GRACE_SECS));
		assert!(!gate.should_unveil(false, 0, false, UNVEIL_ARM_GRACE_SECS));
	}

	#[test]
	fn busy_counter_waits_for_quiet_frames() {
		let mut gate = gate_at(0.0);
		gate.observe(80);
		assert!(gate.saw_work);
		assert!(!gate.should_unveil(true, 80, false, 2.0));
		gate.observe(8);
		assert!(!gate.should_unveil(true, 8, false, 2.0));
		gate.observe(8);
		assert!(gate.should_unveil(true, 8, false, 2.0));
	}

	#[test]
	fn timeout_unveils_only_when_ready() {
		let mut gate = gate_at(0.0);
		gate.observe(400);
		assert!(!gate.should_unveil(false, 400, false, UNVEIL_TIMEOUT_SECS));
		assert!(gate.should_unveil(true, 400, false, UNVEIL_TIMEOUT_SECS));
	}

	#[test]
	fn a_ready_surface_without_a_gate_requests_the_discovery_world() -> anyhow::Result<()> {
		use layer_stack::ActiveGenerationMode;
		use maybraid_game_mode_discover::Discovery;
		let mut world = World::new();
		world.insert_resource(NextState::<GameFlow>::Unchanged);
		world.insert_resource(NextState::<ActiveGenerationMode>::Unchanged);
		world.insert_resource(WorldSurfaceReady(true));
		world.init_resource::<Time>();
		world
			.run_system_once(finish_world_loading)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let flow = world.resource::<NextState<GameFlow>>();
		let mode = world.resource::<NextState<ActiveGenerationMode>>();
		let flow_ok = matches!(flow, NextState::Pending(GameFlow::World));
		let mode_ok = matches!(mode, NextState::PendingIfNeq(mode) if mode.is::<Discovery>());
		if !flow_ok || !mode_ok {
			return Err(anyhow::anyhow!("unveil requested flow {flow:?} mode {mode:?}"));
		}
		Ok(())
	}

	#[test]
	fn work_spike_resets_quiet_frames() {
		let mut gate = gate_at(0.0);
		gate.observe(80);
		gate.observe(4);
		gate.observe(80);
		assert_eq!(gate.quiet_frames, 0);
		assert!(!gate.should_unveil(true, 80, false, 5.0));
	}

	#[test]
	fn grace_does_not_unveil_while_near_work_is_outstanding() {
		let mut gate = gate_at(0.0);
		gate.observe(40);
		assert!(!gate.should_unveil(true, 40, false, UNVEIL_ARM_GRACE_SECS + 1.0));
		assert!(!gate.should_unveil(true, 0, true, UNVEIL_ARM_GRACE_SECS + 1.0));
	}

	#[test]
	fn outstanding_ambient_work_does_not_block_unveil() {
		let gate = gate_at(0.0);
		assert!(
			gate.should_unveil(true, 0, false, UNVEIL_ARM_GRACE_SECS),
			"ambient remaining is not part of active"
		);
	}

	struct NearTile;

	impl lod::hcsg::shared::GenerationScheme for NearTile {
		fn original_ids_for(
			_: &mut lod::hcsg::shared::GenerationContext,
			region: bevy::math::bounding::Aabb3d,
		) -> Vec<lod::gen::OriginalId> {
			let start = region.min.x.floor() as i32;
			let end = region.max.x.ceil() as i32;
			(start..end)
				.map(|x| {
					let bounds = bevy::math::bounding::Aabb3d::from_min_max(
						Vec3::new(x as f32, 0.0, 0.0),
						Vec3::new(x as f32 + 1.0, 1.0, 1.0),
					);
					lod::gen::OriginalId::new(lod::gen::Id::from_cell(bounds))
				})
				.collect()
		}

		fn build_with_id(
			_: &mut lod::hcsg::shared::GenerationContext,
			id: lod::gen::Id,
		) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
			let bounds = id.origin_cell_bounds()?;
			Some((Self, bounds))
		}
	}

	fn gated_world(demand: HcsgDemand) -> World {
		let mut world = World::new();
		world.insert_resource(NextState::<GameFlow>::Unchanged);
		world.insert_resource(NextState::<layer_stack::ActiveGenerationMode>::Unchanged);
		world.insert_resource(WorldSurfaceReady(true));
		world.init_resource::<Time>();
		world.insert_resource(demand);
		world
	}

	fn near_region() -> bevy::math::bounding::Aabb3d {
		bevy::math::bounding::Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 1.0, 1.0))
	}

	fn unveiled(world: &World) -> bool {
		matches!(world.resource::<NextState<GameFlow>>(), NextState::Pending(GameFlow::World))
	}

	#[test]
	fn finish_world_loading_does_not_unveil_while_near_is_outstanding() -> anyhow::Result<()> {
		let demand = HcsgDemand::default();
		demand.subscribe::<NearTile>(None, vec![near_region()], None, HcsgClass::Near);
		let mut world = gated_world(demand);
		world
			.run_system_once(finish_world_loading)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		if unveiled(&world) {
			return Err(anyhow::anyhow!("unveil proceeded while near HCSG work was outstanding"));
		}
		Ok(())
	}

	#[test]
	fn finish_world_loading_unveils_while_only_ambient_is_outstanding() -> anyhow::Result<()> {
		let demand = HcsgDemand::default();
		demand.subscribe::<NearTile>(None, vec![near_region()], None, HcsgClass::Ambient);
		let mut world = gated_world(demand);
		world
			.run_system_once(finish_world_loading)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		if !unveiled(&world) {
			return Err(anyhow::anyhow!("ambient outstanding blocked unveil"));
		}
		Ok(())
	}
}
