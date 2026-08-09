//! Playground CPU / GPU / ApplyDeferred diagnostics.
//!
//! Toggle with env `CHICO_SBS_DIAG` (comma-separated flags):
//! - `fps` — throttled `[veg.timing]` FPS / frame_ms (default when unset)
//! - `commands` — `[lod.commands]` system_commands apply (needs bevy `trace`)
//! - `render` — top `[veg.render]` elapsed_gpu/cpu paths with the FPS line
//! - `lod` — allow `lod` crate `info` (`[lod.fine]` / `[lod.chunk]`); otherwise `lod=warn`
//! - `all` — `fps,commands,render,lod`
//! - `ms=<f64>` — min ms to report for `commands` / `render` (default **1.0**)
//!
//! Or set `CHICO_SBS_DIAG_MS` (same default). Inline `ms=` wins over the dedicated env.
//!
//! Examples:
//! ```text
//! CHICO_SBS_DIAG=fps          # default
//! CHICO_SBS_DIAG=all
//! CHICO_SBS_DIAG=fps,commands,render,ms=1
//! CHICO_SBS_DIAG=all CHICO_SBS_DIAG_MS=2.5
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
const ENV_DIAG_MS: &str = "CHICO_SBS_DIAG_MS";
const LOG_INTERVAL: Duration = Duration::from_secs(1);
const TOP_RENDER_PATHS: usize = 8;
/// Default floor for command-apply and render-path lines (lurch hunt without spam).
const DEFAULT_MIN_MS: f64 = 1.0;

/// Parsed [`CHICO_SBS_DIAG`] flags (also a Bevy [`Resource`] for systems).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct PlaygroundDiag {
	pub fps: bool,
	pub commands: bool,
	pub render: bool,
	/// When false, LogPlugin filter quiets `lod` to `warn` (hides `[lod.fine]` spam).
	pub lod: bool,
	/// Min milliseconds for `[lod.commands]` / `[veg.render]` lines.
	pub min_ms: f64,
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
		let env_ms = parse_min_ms_env();
		if raw.is_empty() {
			return Self::fps_only_with_ms(env_ms.unwrap_or(DEFAULT_MIN_MS));
		}
		let mut diag = Self {
			fps: false,
			commands: false,
			render: false,
			lod: false,
			min_ms: env_ms.unwrap_or(DEFAULT_MIN_MS),
		};
		let mut saw_inline_ms = false;
		for part in raw.split(',') {
			let part = part.trim();
			if part.is_empty() {
				continue;
			}
			let lower = part.to_ascii_lowercase();
			if let Some(ms) = parse_ms_flag(&lower) {
				diag.min_ms = ms;
				saw_inline_ms = true;
				continue;
			}
			match lower.as_str() {
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
					eprintln!(
						"[{ENV_DIAG}] unknown flag {other:?} (use fps,commands,render,lod,all,ms=<f64>)"
					);
				}
			}
		}
		if !saw_inline_ms {
			if let Some(ms) = env_ms {
				diag.min_ms = ms;
			}
		}
		if !diag.fps && !diag.commands && !diag.render && !diag.lod {
			return Self::fps_only_with_ms(diag.min_ms);
		}
		diag
	}

	pub fn fps_only() -> Self {
		Self::fps_only_with_ms(DEFAULT_MIN_MS)
	}

	fn fps_only_with_ms(min_ms: f64) -> Self {
		Self { fps: true, commands: false, render: false, lod: false, min_ms }
	}

	pub fn summary(self) -> String {
		let mut parts = Vec::new();
		if self.fps {
			parts.push("fps".to_string());
		}
		if self.commands {
			parts.push("commands".to_string());
		}
		if self.render {
			parts.push("render".to_string());
		}
		if self.lod {
			parts.push("lod".to_string());
		}
		if parts.is_empty() {
			parts.push("off".to_string());
		}
		parts.push(format!("ms={}", trim_ms(self.min_ms)));
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

fn parse_min_ms_env() -> Option<f64> {
	let raw = std::env::var(ENV_DIAG_MS).ok()?;
	parse_positive_ms(raw.trim())
}

fn parse_ms_flag(flag: &str) -> Option<f64> {
	let value = flag
		.strip_prefix("ms=")
		.or_else(|| flag.strip_prefix("min_ms="))
		.or_else(|| flag.strip_prefix("threshold_ms="))?;
	parse_positive_ms(value)
}

fn parse_positive_ms(raw: &str) -> Option<f64> {
	let ms: f64 = raw.parse().ok()?;
	(ms.is_finite() && ms >= 0.0).then_some(ms)
}

fn trim_ms(ms: f64) -> String {
	let s = format!("{ms:.3}");
	s.trim_end_matches('0').trim_end_matches('.').to_string()
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
	let min_ms = diag.min_ms;
	let mut render: Vec<(&str, f64)> = diagnostics
		.iter()
		.filter_map(|d| {
			let path = d.path().as_str();
			if !(path.contains("elapsed_gpu") || path.contains("elapsed_cpu")) {
				return None;
			}
			let value = d.smoothed().or_else(|| d.value())?;
			(value >= min_ms).then_some((path, value))
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
	let diag = PlaygroundDiag::from_env();
	if !diag.commands {
		return None;
	}
	Some(Box::new(SystemCommandsTimingLayer::new(diag.min_ms)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_inline_ms_flag() {
		assert_eq!(parse_ms_flag("ms=1"), Some(1.0));
		assert_eq!(parse_ms_flag("ms=2.5"), Some(2.5));
		assert_eq!(parse_ms_flag("min_ms=0"), Some(0.0));
		assert_eq!(parse_ms_flag("fps"), None);
		assert_eq!(parse_ms_flag("ms=-1"), None);
	}
}
