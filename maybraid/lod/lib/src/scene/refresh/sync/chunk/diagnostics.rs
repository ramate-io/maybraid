//! Opt-in scene refresh telemetry (default-off feature only).
//!
//! Text logs for drain classification, new-root visits, and first-spawn events.
//! Guards ensure zero counters, allocations, or queries when the feature is off.

use bevy::prelude::*;
use std::time::Duration;

use crate::scene::level::LodSceneLevel;

pub const TARGET: &str = "maybraid.lod.drain";

/// Check if telemetry is enabled (feature must be on AND INFO level active).
#[inline]
pub fn enabled() -> bool {
	#[cfg(feature = "scene-tracing")]
	{
		tracing::enabled!(target: TARGET, tracing::Level::INFO)
	}
	#[cfg(not(feature = "scene-tracing"))]
	{
		false
	}
}

/// Emit drain frame classification summary (only when enabled).
pub fn emit_classification_summary(
	world: &World,
	new_roots_total: u32,
	skipped_queue_empty: u32,
	skipped_no_host: u32,
	skipped_cold_mismatch: u32,
	skipped_not_shown: u32,
	budget_total: u32,
) {
	#[cfg(feature = "scene-tracing")]
	{
		use super::types::LodChunkDrainCursor;
		if enabled() {
			tracing::info!(
				target: TARGET,
				frame = world.resource::<LodChunkDrainCursor>().frame,
				new_roots_total,
				skipped_queue_empty,
				skipped_no_host,
				skipped_cold_mismatch,
				skipped_not_shown,
				budget_total,
				"drain frame classification summary"
			);
		}
	}
	#[cfg(not(feature = "scene-tracing"))]
	{
		let _ = (world, new_roots_total, skipped_queue_empty, skipped_no_host, skipped_cold_mismatch, skipped_not_shown, budget_total);
	}
}

/// Emit cold mismatch skip for a new root (only when enabled).
pub fn emit_cold_mismatch_skip(
	entity: Entity,
	host: Entity,
	self_level: LodSceneLevel,
	desired: LodSceneLevel,
) {
	#[cfg(feature = "scene-tracing")]
	{
		if enabled() {
			tracing::info!(
				target: TARGET,
				root = ?entity,
				host = ?host,
				self_level = ?self_level,
				desired = ?desired,
				"new cold root skipped (cold but self_level != desired)"
			);
		}
	}
	#[cfg(not(feature = "scene-tracing"))]
	{
		let _ = (entity, host, self_level, desired);
	}
}

/// Emit warm root skip for a new root (only when enabled).
pub fn emit_warm_root_skip(
	entity: Entity,
	host: Entity,
	self_level: LodSceneLevel,
	desired: LodSceneLevel,
	shown: bool,
) {
	#[cfg(feature = "scene-tracing")]
	{
		if enabled() {
			tracing::info!(
				target: TARGET,
				root = ?entity,
				host = ?host,
				self_level = ?self_level,
				desired = ?desired,
				shown = shown,
				"new warm root skipped (not desired and not shown)"
			);
		}
	}
	#[cfg(not(feature = "scene-tracing"))]
	{
		let _ = (entity, host, self_level, desired, shown);
	}
}

/// Emit new root visit telemetry (only when enabled).
pub fn emit_new_root_visit(
	world: &World,
	entity: Entity,
	host: Entity,
	parent_desired: LodSceneLevel,
	self_level: Option<LodSceneLevel>,
	host_desired: Option<LodSceneLevel>,
	cold: bool,
	budget_remaining: u32,
	time_left: Duration,
	quanta: u32,
) {
	#[cfg(feature = "scene-tracing")]
	{
		if enabled() {
			tracing::info!(
				target: TARGET,
				root = ?entity,
				host = ?host,
				parent_desired = ?parent_desired,
				self_level = ?self_level,
				host_desired = ?host_desired,
				cold = cold,
				budget_remaining = budget_remaining,
				time_left_us = time_left.as_micros(),
				quanta = quanta,
				"visiting new root (spawned=0)"
			);
		}
	}
	#[cfg(not(feature = "scene-tracing"))]
	{
		let _ = (world, entity, host, parent_desired, self_level, host_desired, cold, budget_remaining, time_left, quanta);
	}
}

/// Emit first spawn completion for a new root (only when enabled).
pub fn emit_first_spawn_complete(
	entity: Entity,
	host: Entity,
	self_level: Option<LodSceneLevel>,
	primitives_left: usize,
	quanta: u32,
) {
	#[cfg(feature = "scene-tracing")]
	{
		if enabled() {
			tracing::info!(
				target: TARGET,
				root = ?entity,
				host = ?host,
				self_level = ?self_level,
				primitives_left = primitives_left,
				quanta = quanta,
				"new root completed first spawn"
			);
		}
	}
	#[cfg(not(feature = "scene-tracing"))]
	{
		let _ = (entity, host, self_level, primitives_left, quanta);
	}
}

/// Frame classification counters (only when telemetry is enabled).
#[derive(Debug, Default)]
pub struct ClassificationCounters {
	pub new_roots_total: u32,
	pub skipped_queue_empty: u32,
	pub skipped_no_host: u32,
	pub skipped_cold_mismatch: u32,
	pub skipped_not_shown: u32,
}

impl ClassificationCounters {
	/// Increment new roots counter (only when enabled).
	#[inline]
	pub fn count_new_root(&mut self) {
		#[cfg(feature = "scene-tracing")]
		if enabled() {
			self.new_roots_total += 1;
		}
	}

	/// Increment skipped queue empty counter (only when enabled).
	#[inline]
	pub fn count_skipped_queue_empty(&mut self) {
		#[cfg(feature = "scene-tracing")]
		if enabled() {
			self.skipped_queue_empty += 1;
		}
	}

	/// Increment skipped no host counter (only when enabled).
	#[inline]
	pub fn count_skipped_no_host(&mut self) {
		#[cfg(feature = "scene-tracing")]
		if enabled() {
			self.skipped_no_host += 1;
		}
	}

	/// Increment skipped cold mismatch counter (only when enabled).
	#[inline]
	pub fn count_skipped_cold_mismatch(&mut self) {
		#[cfg(feature = "scene-tracing")]
		if enabled() {
			self.skipped_cold_mismatch += 1;
		}
	}

	/// Increment skipped not shown counter (only when enabled).
	#[inline]
	pub fn count_skipped_not_shown(&mut self) {
		#[cfg(feature = "scene-tracing")]
		if enabled() {
			self.skipped_not_shown += 1;
		}
	}

	/// Check if there are any new roots to report.
	#[inline]
	pub fn has_new_roots(&self) -> bool {
		#[cfg(feature = "scene-tracing")]
		{
			self.new_roots_total > 0
		}
		#[cfg(not(feature = "scene-tracing"))]
		{
			false
		}
	}
}
