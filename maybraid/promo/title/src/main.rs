//! Title-sequence window. Pass `--output` to write a frame-exact MP4.

use std::path::PathBuf;

use bevy::prelude::*;
use clap::Parser;
use promo_title::{
	window_plugins, Args, CaptureIo, TitleCapturePlugin, TitleSequence, TitleSequenceConfig,
	TitleSequencePlugin,
};

fn main() {
	let args = Args::parse();
	let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
	let mut sequence = TitleSequence::new(args.label.clone());
	sequence.duration = args.duration;
	let capture = args.output.is_some();
	let config = TitleSequenceConfig {
		sequence,
		width: args.width.max(2),
		height: args.height.max(2),
		fps: args.fps.max(1),
		capture,
	};

	let mut app = App::new();
	app.add_plugins(window_plugins(&config, assets.to_string_lossy().into_owned()))
		.add_plugins(TitleSequencePlugin { config: config.clone() });
	if let Some(output) = args.output {
		app.insert_resource(CaptureIo::new(
			output,
			args.ffmpeg,
			args.keep_frames,
			args.warmup_frames,
		))
		.add_plugins(TitleCapturePlugin);
	}
	app.run();
}
