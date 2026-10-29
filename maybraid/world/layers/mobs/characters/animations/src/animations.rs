pub mod beckon;
pub mod bow;
pub mod dorsoventral_undulation;
pub mod face_palm;
pub mod fall;
pub mod fist_pump;
pub mod fixed;
pub mod fixed_tuck;
pub mod flap_from_glide;
pub mod flapping;
pub mod gallop;
pub mod glide_from_flap;
pub mod grounded_gait;
pub mod hand_clap;
pub mod head_nod;
pub mod idle;
pub mod jab;
pub mod land;
pub mod lateral_undulation;
pub mod leap;
pub mod mix;
pub mod power_dash;
pub mod power_strafe;
pub mod prone;
pub mod prone_ascent;
pub mod prone_descent;
pub mod quadruped_bray;
pub mod quadruped_gallop;
pub mod quadruped_idle;
pub mod quadruped_leap;
pub mod quadruped_run;
pub mod quadruped_run_start;
pub mod quadruped_run_stop;
pub mod quadruped_run_to_gallop;
pub mod quadruped_sleep;
pub mod run;
pub mod run_start;
pub mod run_stop;
pub mod run_to_walk;
pub mod salute;
pub mod shrug;
pub mod soaring;
pub mod spot_scan;
pub mod spring;
pub mod squat;
pub mod squat_ascent;
pub mod squat_descent;
pub mod stomp_beat;
pub mod think_again;
pub mod transition;
pub mod tuck;
pub mod tucked_flip;
pub mod two_footed_jump;
pub mod two_footed_tucked_flip;
pub mod upright_leap;
pub mod upright_power_dash;
pub mod upright_run;
pub mod upright_walk;
pub mod victory_wave;
pub mod walk;
pub mod walk_start;
pub mod walk_stop;
pub mod walk_to_run;

pub use dorsoventral_undulation::DorsoventralUndulation;
pub use fall::Fall;
pub use fixed::FixedPosition;
pub use fixed_tuck::FixedTuck;
pub use flapping::Flapping;
pub use gallop::Gallop;
pub use hand_clap::HandClap;
pub use idle::Idle;
pub use jab::{
	Jab, BACKSWING_END, DEFAULT_BACKSWING, DEFAULT_JAB_TARGET, EXTEND_END, HOLD_END as JAB_HOLD_END,
};
pub use land::Land;
pub use lateral_undulation::LateralUndulation;
pub use leap::{
	Leap, LeapPhase, LeapTimeline, AIR_END, LEAP_LAND_BLEND_FRACTION,
	LEAP_RUN_RECOVERY_BLEND_FRACTION, LEAP_TAKEOFF_BLEND_FRACTION, TAKEOFF_END,
};
pub use mix::{burst_envelope, smoothstep, LinearBlend, Mix, Smooth, DEFAULT_BURST_EDGE};
pub use power_dash::{
	PowerDash, BRAKE_END, COMPRESS_END, HOLD_END as POWER_DASH_HOLD_END, PUSH_END,
};
pub use power_strafe::PowerStrafe;
pub use prone::Prone;
pub use quadruped_bray::{EpiphantBray, MOUTH_EMIT_NECK_LOCAL, MOUTH_EMIT_SOCKET, SONIC_EMIT_NORM};
pub use quadruped_gallop::QuadrupedGallop;
pub use quadruped_idle::QuadrupedIdle;
pub use quadruped_leap::QuadrupedLeap;
pub use quadruped_run::{QuadrupedRun, QuadrupedRunPose};
pub use quadruped_run_stop::QuadrupedRunStop;
pub use quadruped_sleep::{
	EpiphantSleepEnter, EpiphantSleepHold, EpiphantWake, SLEEP_ENTER_IMPACT_NORM, WAKE_IMPACT_NORM,
};
pub use run::Run;
pub use soaring::Soaring;
pub use spring::Spring;
pub use squat::{vertical_drop, Squat};
pub use squat_descent::{SquatDescent, DEFAULT_DESCENT_SPEED};
pub use think_again::{
	ThinkAgain, DEFAULT_SPEED as THINK_AGAIN_SPEED, DURATION as THINK_AGAIN_DURATION,
};
pub use transition::{BlendCurve, Transition, TransitionCurve};
pub use tuck::{Tuck, TuckProfile};
pub use tucked_flip::{FlipDirection, TuckedFlip};
pub use two_footed_jump::{
	BallisticJump, JumpSample, JumpSamplingCache, JumpSegment, JumpTiming, TwoFootedJump,
	DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED,
	DEFAULT_SPRING_DURATION, FALL_BLEND_FRACTION, LAND_BLEND_FRACTION, LAND_POSE_BLEND_MAX_SECS,
};
pub use two_footed_tucked_flip::TwoFootedTuckedFlip;
pub use upright_leap::UprightLeap;
pub use upright_power_dash::UprightPowerDash;
pub use upright_run::UprightRun;
pub use upright_walk::UprightWalk;
pub use victory_wave::VictoryWave;
pub use walk::Walk;

pub use beckon::Beckon;
pub use bow::Bow;
pub use face_palm::FacePalm;
pub use fist_pump::FistPump;
pub use flap_from_glide::FlapFromGlide;
pub use glide_from_flap::GlideFromFlap;
pub use grounded_gait::GroundedGait;
pub use head_nod::HeadNod;
pub use prone_ascent::ProneAscent;
pub use prone_descent::ProneDescent;
pub use quadruped_run_start::QuadrupedRunStart;
pub use quadruped_run_to_gallop::QuadrupedRunToGallop;
pub use run_start::RunStart;
pub use run_stop::RunStop;
pub use run_to_walk::RunToWalk;
pub use salute::Salute;
pub use shrug::Shrug;
pub use spot_scan::SpotScan;
pub use squat_ascent::SquatAscent;
pub use stomp_beat::StompBeat;
pub use walk_start::WalkStart;
pub use walk_stop::WalkStop;
pub use walk_to_run::WalkToRun;

pub use walk_start::DEFAULT_WALK_START_SPEED;

pub use walk_stop::DEFAULT_WALK_STOP_SPEED;

pub use run_start::DEFAULT_RUN_START_SPEED;

pub use run_stop::DEFAULT_RUN_STOP_SPEED;

pub use walk_to_run::DEFAULT_WALK_TO_RUN_SPEED;

pub use run_to_walk::DEFAULT_RUN_TO_WALK_SPEED;

pub use quadruped_run_start::DEFAULT_QUADRUPED_RUN_START_SPEED;

pub use quadruped_run_to_gallop::DEFAULT_QUADRUPED_RUN_TO_GALLOP_SPEED;
pub use run_stop::{
	run_stop_progress_from_speed, speed_for_run_stop_progress, RunStopSegment,
	RUN_STOP_HANDOFF_SPEED, RUN_TO_WALK_END,
};

pub use run_stop::{run_stop_cycle_speed, run_stop_foot_travel_per_cycle};

pub mod crouch_walk;
pub use crouch_walk::CrouchedWalk;

pub mod sit;
pub use sit::Sit;

pub mod lay;
pub use lay::Lay;

pub mod quadruped_sit;
pub use quadruped_sit::QuadrupedSit;

pub mod quadruped_lie_down;
pub use lay::LayOrientation;
pub use quadruped_lie_down::QuadrupedLieDown;
