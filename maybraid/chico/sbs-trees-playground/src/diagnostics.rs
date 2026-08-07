//! Playground CPU / GPU / ApplyDeferred diagnostics.
//!
//! Toggle with env `CHICO_SBS_DIAG` (comma-separated flags):
//! - `fps` — throttled `[veg.timing]` FPS / frame_ms (default when unset)
//! - `commands` — `[lod.commands]` system_commands apply (≥0.25ms; needs bevy `trace`)
//! - `render` — top `[veg.render]` elapsed_gpu/cpu paths with the FPS line
//! - `lod` — allow `lod` crate `info` (`[lod.fine]` / `[lod.chunk]`); otherwise `lod=warn`
//! - `all` — `fps,commands,render,lod`
//!
//! Examples:
//! ```text
//! CHICO_SBS_DIAG=fps          # default
//! CHICO_SBS_DIAG=all
//! CHICO_SBS_DIAG=fps,commands,render
//! ```

use std::time::{Duration, Instant};

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::log::tracing::field::{Field, Visit};
use bevy::log::tracing::span::{Attributes, Id};
use bevy::log::tracing::Subscriber;
use bevy::log::tracing_subscriber::layer::Context;
use bevy::log::tracing_subscriber::registry::LookupSpan;
use bevy::log::tracing_subscriber::Layer;
use bevy::log::BoxedLayer;
use bevy::prelude::*;

const ENV_DIAG: &str = "CHICO_SBS_DIAG";
const LOG_INTERVAL: Duration = Duration::from_secs(1);
const TOP_RENDER_PATHS: usize = 8;
const RENDER_MIN_MS: f64 = 0.25;
const COMMANDS_THRESHOLD_MS: f64 = 0.25;

/// Parsed [`CHICO_SBS_DIAG`] flags (also a Bevy [`Resource`] for systems).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaygroundDiag {
	pub fps: bool,
	pub commands: bool,
	pub render: bool,
	/// When false, LogPlugin filter quiets `lod` to `warn` (hides `[lod.fine]` spam).
	pub lod: bool,
}

impl Default for PlaygroundDiag {
	fn default() -> Self {
		Self::from_env()
	}
}

impl PlaygroundDiag {
	pub fn from_env() -> Self {
		let raw = std::env::var(ENV_DIAG).unwrap_or_default();
		let raw = raw.trim();
		if raw.is_empty() {
			return Self::fps_only();
		}
		let mut diag = Self {
			fps: false,
			commands: false,
			render: false,
			lod: false,
		};
		for part in raw.split(',') {
			match part.trim().to_ascii_lowercase().as_str() {
				"" => {}
				"all" => {
					diag.fps = true;
					diag.commands = true;
					diag.render = true;
					diag.lod = true;
				}
				"fps" => diag.fps = true,
				"commands" | "command" | "apply" => diag.commands = true,
				"render" | "gpu" => diag.render = true,
				"lod" | "fine" => diag.lod = true,
				"timing" => {
					// Alias: frame line + render breakdown.
					diag.fps = true;
					diag.render = true;
				}
				other => {
					eprintln!("[{ENV_DIAG}] unknown flag {other:?} (use fps,commands,render,lod,all)");
				}
			}
		}
		if !diag.fps && !diag.commands && !diag.render && !diag.lod {
			return Self::fps_only();
		}
		diag
	}

	pub fn fps_only() -> Self {
		Self { fps: true, commands: false, render: false, lod: false }
	}

	pub fn summary(self) -> String {
		let mut parts = Vec::new();
		if self.fps {
			parts.push("fps");
		}
		if self.commands {
			parts.push("commands");
		}
		if self.render {
			parts.push("render");
		}
		if self.lod {
			parts.push("lod");
		}
		if parts.is_empty() {
			parts.push("off");
		}
		format!("{ENV_DIAG}={}", parts.join(","))
	}

	/// Bevy `LogPlugin` filter: quiet `lod` info unless `lod` is enabled.
	pub fn log_filter(self) -> String {
		let base = std::env::var("RUST_LOG")
			.unwrap_or_else(|_| bevy::log::DEFAULT_FILTER.to_string());
		if self.lod || base.contains("lod=") {
			base
		} else {
			format!("{base},lod=warn")
		}
	}
}

pub struct PlaygroundTimingPlugin;

impl Plugin for PlaygroundTimingPlugin {
	fn build(&self, app: &mut App) {
		let diag = PlaygroundDiag::from_env();
		app.insert_resource(diag);
		if diag.fps || diag.render {
			if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
				app.add_plugins(FrameTimeDiagnosticsPlugin::default());
			}
			app.add_systems(Update, log_frame_and_render_timing);
		}
	}
}

fn log_frame_and_render_timing(
	time: Res<Time>,
	diag: Res<PlaygroundDiag>,
	mut accum: Local<Duration>,
	diagnostics: Res<DiagnosticsStore>,
) {
	if !diag.fps && !diag.render {
		return;
	}
	*accum += time.delta();
	if *accum < LOG_INTERVAL {
		return;
	}
	*accum = Duration::ZERO;

	if diag.fps {
		let fps = diagnostics
			.get(&FrameTimeDiagnosticsPlugin::FPS)
			.and_then(|d| d.smoothed())
			.unwrap_or(f64::NAN);
		let frame_ms = diagnostics
			.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
			.and_then(|d| d.smoothed())
			.unwrap_or(f64::NAN);
		eprintln!("[veg.timing] fps={fps:.1} frame_ms={frame_ms:.2}");
	}

	if !diag.render {
		return;
	}
	let mut render: Vec<(&str, f64)> = diagnostics
		.iter()
		.filter_map(|d| {
			let path = d.path().as_str();
			if !(path.contains("elapsed_gpu") || path.contains("elapsed_cpu")) {
				return None;
			}
			let value = d.smoothed().or_else(|| d.value())?;
			(value >= RENDER_MIN_MS).then_some((path, value))
		})
		.collect();
	render.sort_by(|a, b| b.1.total_cmp(&a.1));
	for (path, ms) in render.into_iter().take(TOP_RENDER_PATHS) {
		eprintln!("[veg.render] {path}={ms:.2}ms");
	}
}

/// Wall time from last enter of a `system_commands` span.
struct CommandApplyTimer(Instant);

struct SystemName(String);

/// Logs Bevy `system_commands` spans (command / archetype apply) above a threshold.
struct SystemCommandsTimingLayer {
	threshold_ms: f64,
}

impl SystemCommandsTimingLayer {
	fn new(threshold_ms: f64) -> Self {
		Self { threshold_ms }
	}
}

impl<S> Layer<S> for SystemCommandsTimingLayer
where
	S: Subscriber + for<'a> LookupSpan<'a>,
{
	fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
		if attrs.metadata().name() != "system_commands" {
			return;
		}
		let Some(span) = ctx.span(id) else {
			return;
		};
		let mut name = SystemNameVisitor::default();
		attrs.record(&mut name);
		let mut ext = span.extensions_mut();
		if let Some(n) = name.0 {
			ext.insert(SystemName(n));
		}
		ext.insert(CommandApplyTimer(Instant::now()));
	}

	fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
		let Some(span) = ctx.span(id) else {
			return;
		};
		if span.metadata().name() != "system_commands" {
			return;
		}
		span.extensions_mut().replace(CommandApplyTimer(Instant::now()));
	}

	fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
		let Some(span) = ctx.span(id) else {
			return;
		};
		if span.metadata().name() != "system_commands" {
			return;
		}
		let ext = span.extensions();
		let Some(CommandApplyTimer(start)) = ext.get::<CommandApplyTimer>() else {
			return;
		};
		let ms = start.elapsed().as_secs_f64() * 1000.0;
		if ms < self.threshold_ms {
			return;
		}
		let name = ext
			.get::<SystemName>()
			.map(|n| n.0.as_str())
			.unwrap_or("<unknown>");
		eprintln!("[lod.commands] system_commands apply={ms:.2}ms system={name}");
	}
}

#[derive(Default)]
struct SystemNameVisitor(Option<String>);

impl Visit for SystemNameVisitor {
	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		if field.name() == "name" {
			self.0 = Some(format!("{value:?}"));
		}
	}

	fn record_str(&mut self, field: &Field, value: &str) {
		if field.name() == "name" {
			self.0 = Some(value.to_owned());
		}
	}
}

/// `LogPlugin::custom_layer` callback — installs command-apply timing when enabled.
pub fn command_apply_timing_layer(_app: &mut App) -> Option<BoxedLayer> {
	if !PlaygroundDiag::from_env().commands {
		return None;
	}
	Some(Box::new(SystemCommandsTimingLayer::new(COMMANDS_THRESHOLD_MS)))
}
