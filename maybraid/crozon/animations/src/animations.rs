pub mod fall;
pub mod land;
pub mod mix;
pub mod run;
pub mod spring;
pub mod squat;
pub mod transition;
pub mod two_footed_jump;

pub use fall::Fall;
pub use land::Land;
pub use mix::{smoothstep, Mix, Smooth};
pub use run::Run;
pub use spring::Spring;
pub use squat::{Squat, vertical_drop};
pub use transition::{BlendCurve, Transition, TransitionCurve};
pub use two_footed_jump::{
	air_duration, ballistic_height, launch_speed, JumpSegment, JumpTiming, TwoFootedJump,
	DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED,
	DEFAULT_SPRING_DURATION, FALL_BLEND_FRACTION, LAND_BLEND_FRACTION, LAND_POSE_BLEND_MAX_SECS,
};
