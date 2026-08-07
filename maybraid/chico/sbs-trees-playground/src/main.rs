use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::log::tracing::field::{Field, Visit};
use bevy::log::tracing::span::{Attributes, Id};
use bevy::log::tracing::Subscriber;
use bevy::log::tracing_subscriber::layer::Context;
use bevy::log::tracing_subscriber::registry::LookupSpan;
use bevy::log::tracing_subscriber::Layer;
use bevy::log::{BoxedLayer, LogPlugin};
use bevy::prelude::*;
use chico_sbs_trees_playground::{
	PendingStartupCommand, PlaygroundCommand, SbsTreesPlaygroundPlugin,
};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

/// Wall time from last enter of a `system_commands` span.
struct CommandApplyTimer(Instant);

struct SystemName(String);

/// Logs Bevy's built-in `system_commands` spans (command / archetype apply) when
/// they exceed a small threshold — same wiring as richmond-buildings-playground /
/// [PR #595](https://github.com/ramate-io/maybraid/pull/595).
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
		// eprintln avoids re-entrancy through the logging subscriber.
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

fn command_apply_timing_layer(_app: &mut App) -> Option<BoxedLayer> {
	Some(Box::new(SystemCommandsTimingLayer::new(0.25)))
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|e| {
		eprintln!("{e}");
		std::process::exit(2);
	});
	if startup.is_some() {
		println!("Startup command from argv (same as in-game / text).");
	} else {
		println!("SBS trees playground — press / for commands.");
	}
	println!(
		"Timing: [lod.commands] ApplyDeferred/system_commands (≥0.25ms); [veg.timing]/[veg.render] every 1s."
	);

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Chico SBS Trees Playground".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() })
				.set(LogPlugin {
					custom_layer: command_apply_timing_layer,
					..default()
				}),
		)
		.insert_resource(ClearColor(Color::srgb(0.82, 0.88, 0.92)))
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(SbsTreesPlaygroundPlugin)
		.run();
}
