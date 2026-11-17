//! Opt-in pipeline identity and readiness evidence, at render-frame granularity.
//!
//! This observes the existing cache; it neither compiles extra pipelines nor
//! claims that the interval between observations measures compilation time.

use bevy::prelude::*;
use bevy::render::{
	render_resource::{CachedPipelineState, PipelineCache},
	Render, RenderApp, RenderSystems,
};

pub struct PipelineProfilePlugin;

impl Plugin for PipelineProfilePlugin {
	fn build(&self, app: &mut App) {
		if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
			render_app.add_systems(Render, observe.after(RenderSystems::Render));
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
	Queued,
	Creating,
	Ready,
	Error,
}

impl Status {
	fn of(state: &CachedPipelineState) -> Self {
		match state {
			CachedPipelineState::Queued => Self::Queued,
			CachedPipelineState::Creating(_) => Self::Creating,
			CachedPipelineState::Ok(_) => Self::Ready,
			CachedPipelineState::Err(_) => Self::Error,
		}
	}
}

#[derive(Default)]
struct Observed {
	frame: u64,
	epoch: u64,
	states: Vec<Status>,
}

impl Observed {
	fn record(&mut self, index: usize, state: Status) -> bool {
		if let Some(previous) = self.states.get_mut(index) {
			if *previous == state {
				return false;
			}
			*previous = state;
		} else {
			// The public cache iterator visits its contiguous pipeline vector.
			self.states.push(state);
		}
		true
	}
}

fn observe(cache: Res<PipelineCache>, mut observed: Local<Observed>) {
	if !bevy::log::tracing::enabled!(target: "maybraid.pipeline.profile", bevy::log::tracing::Level::TRACE)
	{
		return;
	}
	let _span =
		bevy::log::trace_span!(target: "maybraid.pipeline.profile", "pipeline_profile.observe")
			.entered();
	observed.frame += 1;
	if cache.is_added() {
		observed.epoch += 1;
		observed.states.clear();
	}
	let mut changed = 0;
	for (index, pipeline) in cache.pipelines().enumerate() {
		let first_seen = index >= observed.states.len();
		let state = Status::of(&pipeline.state);
		if observed.record(index, state) {
			changed += 1;
			if first_seen {
				// Full descriptors include labels, shader handles/paths, entry points,
				// definitions and render-state variants. Identity is local to this epoch.
				bevy::log::trace!(target: "maybraid.pipeline.profile",
					frame = observed.frame, epoch = observed.epoch, pipeline = index,
					descriptor = ?pipeline.descriptor, "pipeline_profile.variant");
			}
			bevy::log::trace!(target: "maybraid.pipeline.profile",
				frame = observed.frame, epoch = observed.epoch, pipeline = index,
				?state, "pipeline_profile.state");
		}
	}
	if changed > 0 {
		let ready = observed.states.iter().filter(|&&state| state == Status::Ready).count();
		bevy::log::trace!(target: "maybraid.pipeline.profile",
			frame = observed.frame, epoch = observed.epoch, changed,
			total = observed.states.len(), ready,
			waiting = cache.waiting_pipelines().count(), "pipeline_profile.summary");
	}
}
