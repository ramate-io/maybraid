use bevy::prelude::*;
use character_animations::animations::quadruped_run_stop::DEFAULT_QUADRUPED_RUN_STOP_SPEED;
use character_animations::animations::{
	DEFAULT_BACKSWING, DEFAULT_JAB_TARGET, THINK_AGAIN_DURATION, THINK_AGAIN_SPEED,
};
use character_rigs::{articulation::compose_parent_rotation, authoring::humanoid_bone_axis, Side};
use characters::{AnimBone, AnimClip, AnimRef, JabParams};
use clap::ValueEnum;

use crate::character::CharacterConfig;

/// Humanoid clips the playground can sample. Mirrors the mid-August concepts
/// `--animation` catalog plus playground-only squat / tuck variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum AnimationMode {
	Still,
	Walk,
	#[default]
	Run,
	Squat,
	SquatDescent,
	Jump,
	Leap,
	Tuck,
	FixedTuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
	Soaring,
	Flapping,
	Jab,
	ThinkAgain,
	Beckon,
	Bow,
	FacePalm,
	FistPump,
	HeadNod,
	Salute,
	Shrug,
	SpotScan,
	StompBeat,
	WalkStart,
	WalkStop,
	RunStart,
	RunStop,
	WalkToRun,
	RunToWalk,
	QuadrupedRunStart,
	QuadrupedRunToGallop,
	ProneDescent,
	ProneAscent,
	SquatAscent,
	FlapFromGlide,
	GlideFromFlap,
	Sit,
	Seat,
	Lay,
	LaySide,
	QuadrupedSit,
	QuadrupedLieDown,
	QuadrupedCurl,
	CrouchWalk,
	GroundedGait,
	VictoryWave,
	HandClap,
	QuadrupedRunStop,
	PowerStrafe,
	PowerDash,
	Prone,
}

impl AnimationMode {
	pub const fn uses_side(self) -> bool {
		matches!(
			self,
			Self::Jab
				| Self::ThinkAgain
				| Self::PowerStrafe
				| Self::VictoryWave
				| Self::LaySide
				| Self::FistPump
				| Self::Salute
				| Self::FacePalm
				| Self::StompBeat
				| Self::Beckon
		)
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Still => "still",
			Self::Walk => "walk",
			Self::Run => "run",
			Self::Squat => "squat",
			Self::SquatDescent => "squat-descent",
			Self::Jump => "jump",
			Self::Leap => "leap",
			Self::Tuck => "tuck",
			Self::FixedTuck => "fixed-tuck",
			Self::TuckedFlip => "tucked-flip",
			Self::TwoFootedTuckedFlip => "two-footed-tucked-flip",
			Self::Soaring => "soaring",
			Self::Flapping => "flapping",
			Self::Jab => "jab",
			Self::ThinkAgain => "think-again",
			Self::Beckon => "beckon",
			Self::Bow => "bow",
			Self::FacePalm => "face-palm",
			Self::FistPump => "fist-pump",
			Self::HeadNod => "head-nod",
			Self::Salute => "salute",
			Self::Shrug => "shrug",
			Self::SpotScan => "spot-scan",
			Self::StompBeat => "stomp-beat",
			Self::WalkStart => "walk-start",
			Self::WalkStop => "walk-stop",
			Self::RunStart => "run-start",
			Self::RunStop => "run-stop",
			Self::WalkToRun => "walk-to-run",
			Self::RunToWalk => "run-to-walk",
			Self::QuadrupedRunStart => "quadruped-run-start",
			Self::QuadrupedRunToGallop => "quadruped-run-to-gallop",
			Self::ProneDescent => "prone-descent",
			Self::ProneAscent => "prone-ascent",
			Self::SquatAscent => "squat-ascent",
			Self::FlapFromGlide => "flap-from-glide",
			Self::GlideFromFlap => "glide-from-flap",
			Self::Sit => "sit",
			Self::Seat => "seat",
			Self::Lay => "lay",
			Self::LaySide => "lay-side",
			Self::QuadrupedSit => "quadruped-sit",
			Self::QuadrupedLieDown => "quadruped-lie-down",
			Self::QuadrupedCurl => "quadruped-curl",
			Self::CrouchWalk => "crouch-walk",
			Self::GroundedGait => "grounded-gait",
			Self::VictoryWave => "victory-wave",
			Self::HandClap => "hand-clap",
			Self::QuadrupedRunStop => "quadruped-run-stop",
			Self::PowerStrafe => "power-strafe",
			Self::PowerDash => "power-dash",
			Self::Prone => "prone",
		}
	}

	pub fn anim_clip(self, side: Side) -> AnimClip {
		match self {
			Self::Still => AnimClip::still(),
			Self::Walk => AnimClip::walk(),
			Self::Run => AnimClip::run(),
			Self::Squat => AnimClip::squat(),
			Self::SquatDescent => AnimClip::squat_descent(),
			Self::Jump => AnimClip::jump(),
			Self::Leap => AnimClip::leap(),
			Self::Tuck | Self::FixedTuck => AnimClip::tuck(),
			Self::TuckedFlip => AnimClip::tucked_flip(),
			Self::TwoFootedTuckedFlip => AnimClip::two_footed_tucked_flip(),
			Self::Soaring => AnimClip::soaring(),
			Self::Flapping => AnimClip::flapping(),
			Self::Jab => AnimClip::Jab(JabParams {
				side,
				backswing: DEFAULT_BACKSWING,
				target: DEFAULT_JAB_TARGET,
			}),
			Self::ThinkAgain => AnimClip::think_again_side(side),
			Self::Beckon => AnimClip::beckon(side),
			Self::Bow => AnimClip::bow(),
			Self::FacePalm => AnimClip::face_palm(side),
			Self::FistPump => AnimClip::fist_pump(side),
			Self::HeadNod => AnimClip::head_nod(),
			Self::Salute => AnimClip::salute(side),
			Self::Shrug => AnimClip::shrug(),
			Self::SpotScan => AnimClip::spot_scan(),
			Self::StompBeat => AnimClip::stomp_beat(side),
			Self::WalkStart => AnimClip::walk_start(),
			Self::WalkStop => AnimClip::walk_stop(),
			Self::RunStart => AnimClip::run_start(),
			Self::RunStop => AnimClip::run_stop(),
			Self::WalkToRun => AnimClip::walk_to_run(),
			Self::RunToWalk => AnimClip::run_to_walk(),
			Self::QuadrupedRunStart => AnimClip::quadruped_run_start(),
			Self::QuadrupedRunToGallop => AnimClip::quadruped_run_to_gallop(0.0),
			Self::ProneDescent => AnimClip::prone_descent(),
			Self::ProneAscent => AnimClip::prone_ascent(),
			Self::SquatAscent => AnimClip::squat_ascent(),
			Self::FlapFromGlide => AnimClip::flap_from_glide(),
			Self::GlideFromFlap => AnimClip::glide_from_flap(),
			Self::Sit => AnimClip::sit(),
			Self::Seat => AnimClip::seat(),
			Self::Lay => AnimClip::lay(),
			Self::LaySide => AnimClip::lay_side(side),
			Self::QuadrupedSit => AnimClip::quadruped_sit(),
			Self::QuadrupedLieDown => AnimClip::quadruped_lie_down(),
			Self::QuadrupedCurl => AnimClip::quadruped_curl(),
			Self::CrouchWalk => AnimClip::crouch_walk(),
			Self::GroundedGait => AnimClip::grounded_gait(1.0, 0.5),
			Self::VictoryWave => AnimClip::victory_wave(side),
			Self::HandClap => AnimClip::hand_clap(),
			Self::QuadrupedRunStop => AnimClip::quadruped_run_stop(),
			Self::PowerStrafe => AnimClip::power_strafe(side),
			Self::PowerDash => AnimClip::power_dash(),
			Self::Prone => AnimClip::prone(),
		}
	}

	pub fn anim_ref(self, side: Side) -> AnimRef {
		AnimRef::new(self.anim_clip(side))
	}

	/// Mailbox sample coordinate. Held tuck stays at full fold.
	///
	/// One-shots clamp in the clip. When the playground is looping, wrap on
	/// [`Self::clip_duration`] so `think-again` repeats all four beats.
	pub fn mailbox_progress(self, elapsed: f32, looping: bool) -> f32 {
		if matches!(self, Self::FixedTuck) {
			return 1.0;
		}
		let duration = self.clip_duration();
		if looping && self.wraps_when_looping() {
			elapsed.rem_euclid(duration)
		} else if self.wraps_when_looping() {
			elapsed.min(duration)
		} else {
			elapsed
		}
	}

	pub fn clip_duration(self) -> f32 {
		match self {
			Self::Jump => preview_jump_sampler()
				.timings(character_rigs::humanoid::LegSegmentLengths::default())
				.cycle_duration(),
			Self::ThinkAgain => THINK_AGAIN_DURATION,
			_ => 1.0,
		}
	}

	pub const fn clip_speed(self) -> f32 {
		match self {
			Self::ThinkAgain => THINK_AGAIN_SPEED,
			Self::WalkStart => 3.5,
			Self::WalkStop => 3.5,
			Self::RunStart => 4.5,
			Self::RunStop => 2.5,
			Self::WalkToRun => 4.0,
			Self::RunToWalk => 4.0,
			Self::QuadrupedRunStart => 3.5,
			Self::QuadrupedRunToGallop => 4.0,
			Self::ProneDescent => 2.85,
			Self::ProneAscent => 2.85,
			Self::SquatAscent => 2.85,
			Self::SpotScan => 0.87,
			Self::CrouchWalk => 1.08,
			Self::GroundedGait => 1.38,

			Self::QuadrupedRunStop => DEFAULT_QUADRUPED_RUN_STOP_SPEED,
			_ => 1.0,
		}
	}

	const fn wraps_when_looping(self) -> bool {
		!matches!(
			self,
			Self::Sit
				| Self::Seat | Self::Lay
				| Self::LaySide
				| Self::QuadrupedSit
				| Self::QuadrupedLieDown
				| Self::QuadrupedCurl
				| Self::Still
				| Self::Soaring
				| Self::Flapping
				| Self::FlapFromGlide
				| Self::GlideFromFlap
		)
	}
}

/// Use the same public jump knobs as the mailbox sampler, including both windup halves.
fn preview_jump_sampler() -> character_animations::animations::TwoFootedJump {
	let params = characters::JumpParams::default();
	character_animations::animations::TwoFootedJump::default()
		.with_gravity(params.gravity)
		.with_jump_height(params.jump_height)
		.with_pre_squat_speed(params.pre_squat_speed)
		.with_landing_squat_speed(params.landing_squat_speed)
}

#[derive(Resource, Debug, Clone)]
pub struct AnimationPlayback {
	pub paused: bool,
	pub speed: f32,
	/// When false, the sample clock stops at the clip duration.
	pub looping: bool,
	pub elapsed: f32,
	/// Set by `/character playback --progress`. Consumed on the next sample.
	pub scrub: Option<f32>,
	/// Added to the sample clock. `0.5` is the opposite gait phase.
	pub phase: f32,
	/// Scales femur and shin rest translations before sampling.
	pub leg_scale: f32,
	/// Bone name for the axis gizmo. Empty hides it.
	pub joint: String,
	/// Draw the effective-rest direction next to the posed bone.
	pub show_rest: bool,
	/// When set, replace that bone's clip rotation with these anatomical degrees.
	pub joint_degrees: Option<JointDegrees>,
}

#[derive(Debug, Clone, Copy)]
pub struct JointDegrees {
	pub flexion: f32,
	pub lateral: f32,
	pub axial: f32,
}

impl Default for AnimationPlayback {
	fn default() -> Self {
		Self {
			paused: false,
			speed: 1.0,
			looping: true,
			elapsed: 0.0,
			scrub: None,
			phase: 0.0,
			leg_scale: 1.0,
			joint: String::new(),
			show_rest: true,
			joint_degrees: None,
		}
	}
}

impl AnimationPlayback {
	pub fn advance(&mut self, delta_seconds: f32, duration: f32, clip_speed: f32) {
		if let Some(progress) = self.scrub.take() {
			self.elapsed = progress.max(0.0);
			return;
		}
		if self.paused {
			return;
		}
		self.elapsed += delta_seconds * self.speed * clip_speed;
		if !self.looping {
			self.elapsed = self.elapsed.min(duration);
		}
	}
}

pub fn apply_joint_preview(
	playback: Res<AnimationPlayback>,
	mut bones: Query<(&mut Transform, &AnimBone)>,
) {
	let Some(degrees) = playback.joint_degrees else {
		return;
	};
	if playback.joint.is_empty() {
		return;
	}
	for (mut transform, bone) in &mut bones {
		if bone.name.as_str() != playback.joint {
			continue;
		}
		transform.rotation = compose_parent_rotation(
			bone.rest.rotation,
			humanoid_bone_axis(&playback.joint),
			degrees.lateral.to_radians(),
			degrees.flexion.to_radians(),
			degrees.axial.to_radians(),
		);
	}
}

/// Bone-local axes are bright RGB. Parent-local axes are shorter and dimmer.
/// Character-space axes sit on the rig: +X right, +Y up, +Z fight-forward.
pub fn draw_authoring_gizmos(
	playback: Res<AnimationPlayback>,
	mut gizmos: Gizmos,
	bones: Query<(&GlobalTransform, &Transform, &AnimBone)>,
	config: Res<CharacterConfig>,
) {
	if playback.joint.is_empty() {
		return;
	}
	let origin = config.transform.translation;
	let character = config.transform.rotation;
	draw_axes(&mut gizmos, origin, character, 0.4, 1.0);
	let opposite = opposite_bone(&playback.joint);
	for (global, local, bone) in &bones {
		let selected = bone.name.as_str() == playback.joint;
		let compared = opposite.as_deref() == Some(bone.name.as_str());
		if !selected && !compared {
			continue;
		}
		let origin = global.translation();
		let gain = if selected { 1.0 } else { 0.55 };
		let length = if selected { 0.22 } else { 0.16 };
		draw_axes(&mut gizmos, origin, global.rotation(), length, gain);
		let parent = global.rotation() * local.rotation.inverse();
		draw_axes(&mut gizmos, origin, parent, length * 0.65, gain * 0.45);
		if playback.show_rest {
			let rest_dir = parent * bone.rest.rotation * Vec3::Y * 0.28;
			let color =
				if selected { Color::srgb(1.0, 0.85, 0.2) } else { Color::srgb(0.75, 0.6, 0.15) };
			gizmos.line(origin, origin + rest_dir, color);
		}
		let segment = bone.rest.translation.length().max(0.05);
		let end = origin + global.rotation() * Vec3::Y * segment;
		gizmos.line(end - Vec3::X * 0.03, end + Vec3::X * 0.03, Color::srgb(1.0, 1.0, 1.0));
		gizmos.line(end - Vec3::Z * 0.03, end + Vec3::Z * 0.03, Color::srgb(1.0, 1.0, 1.0));
	}
}

fn opposite_bone(name: &str) -> Option<String> {
	if let Some(base) = name.strip_suffix(".L") {
		Some(format!("{base}.R"))
	} else {
		name.strip_suffix(".R").map(|base| format!("{base}.L"))
	}
}

fn draw_axes(gizmos: &mut Gizmos, origin: Vec3, rotation: Quat, length: f32, gain: f32) {
	gizmos.line(
		origin,
		origin + rotation * Vec3::X * length,
		Color::srgb(gain, 0.15 * gain, 0.15 * gain),
	);
	gizmos.line(
		origin,
		origin + rotation * Vec3::Y * length,
		Color::srgb(0.15 * gain, gain, 0.15 * gain),
	);
	gizmos.line(
		origin,
		origin + rotation * Vec3::Z * length,
		Color::srgb(0.2 * gain, 0.35 * gain, gain),
	);
}

#[cfg(test)]
mod catalog_tests {
	use super::*;
	#[test]
	fn jump_once_reaches_the_full_landing_timeline() {
		let duration = AnimationMode::Jump.clip_duration();
		use character_animations::Animation;
		use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
		let sampler = preview_jump_sampler();
		let mut initial = HumanoidV0Rig::for_clip_test();
		let initial_effect = sampler.apply(&mut initial, 0.0);
		for elapsed in [duration, duration + 10.0] {
			let mut sampled = HumanoidV0Rig::for_clip_test();
			let sample_time = AnimationMode::Jump.mailbox_progress(elapsed, false);
			let effect = sampler.apply(&mut sampled, sample_time);
			assert!(
				effect.0.translation.distance(initial_effect.0.translation) < 1e-4,
				"jump fully recovers its offset"
			);
			for (actual, expected) in sampled.pose.local.iter().zip(&initial.pose.local) {
				assert!(actual.translation.distance(expected.translation) < 1e-4);
				assert!(
					actual.rotation.normalize().dot(expected.rotation.normalize()).abs()
						> 1.0 - 1e-5,
					"jump fully recovers its pose"
				);
			}
		}
		assert!(duration > 1.0, "jump timeline includes windup, flight, and landing");
		let mut playback = AnimationPlayback { looping: false, ..Default::default() };
		playback.advance(100.0, duration, AnimationMode::Jump.clip_speed());
		assert_eq!(playback.elapsed, duration);
		assert_eq!(AnimationMode::Jump.mailbox_progress(duration + 1.0, false), duration);
		assert!((AnimationMode::Jump.mailbox_progress(duration + 0.2, true) - 0.2).abs() < 1e-5);
	}

	#[test]
	fn continuous_preview_clocks_and_grounded_midblend_match_their_samplers() {
		assert_eq!(
			AnimationMode::GroundedGait.anim_clip(Side::Left),
			AnimClip::grounded_gait(1.0, 0.5)
		);
		assert!(
			(AnimationMode::GroundedGait.clip_speed()
				- AnimClip::grounded_gait(1.0, 0.5).default_speed())
			.abs() < 1e-6
		);
		for mode in [
			AnimationMode::Sit,
			AnimationMode::Seat,
			AnimationMode::Lay,
			AnimationMode::LaySide,
			AnimationMode::QuadrupedSit,
			AnimationMode::QuadrupedLieDown,
			AnimationMode::QuadrupedCurl,
			AnimationMode::FlapFromGlide,
			AnimationMode::GlideFromFlap,
		] {
			assert_eq!(mode.mailbox_progress(3.7, true), 3.7);
		}
	}
}
