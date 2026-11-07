//! Scripted Maybraid promotional title sequence.
//!
//! Timeline math is pure and unit-tested. The Bevy plugin reuses the furniture
//! rockadder / cosmos shaders, the HUD Maybraid mark, and a centered
//! [`menu_components::HudTextCard`] for the configurable label.
//! Capture writes a fixed-timestep PNG sequence, an H.264 preview, and a
//! ProRes 4444 overlay MOV whose expanding circular swallow is straight alpha.

pub mod args;
pub mod capture;
pub mod encode;
pub mod plugin;
pub mod timeline;

pub use args::Args;
pub use capture::{CaptureIo, TitleCapturePlugin};
pub use encode::{
	apply_swallow_alpha, composite_over_checkerboard, encode_overlay_sequence, encode_png_sequence,
	ffmpeg_args, ffmpeg_overlay_args,
};
pub use plugin::{
	window_plugins, SequenceClock, TitleSequenceConfig, TitleSequencePlugin, TitleSequenceSystems,
};
pub use timeline::{
	Celestial, FieldStar, Keyframe, LabelPose, NebulaWash, SparkMote, StarPose, Swallow,
	TitleFrame, TitleSequence, TrailBead, DEFAULT_FPS, DEFAULT_HEIGHT, DEFAULT_WIDTH,
	DURATION_SECS, FIELD_STAR_COUNT, NEBULA_COUNT, SPARK_COUNT, STAR_SIZE_PX, TRAIL_BEADS,
};
