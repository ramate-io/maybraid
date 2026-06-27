pub mod fall;
pub mod land;
pub mod mix;
pub mod run;
pub mod spring;
pub mod squat;
pub mod two_footed_jump;

pub use fall::Fall;
pub use land::{Land, DEFAULT_RECOVERY_SPEED};
pub use mix::{smoothstep, Mix, Smooth};
pub use run::Run;
pub use spring::Spring;
pub use squat::{Squat, DEFAULT_WINDUP_DESCENT_SPEED, vertical_drop};
pub use two_footed_jump::{
	air_duration, ballistic_height, launch_speed, suggest_recovery, suggest_windup_descent,
	JumpSegment, JumpSquatTuning, JumpTiming, TwoFootedJump, DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT,
	DEFAULT_SPRING_DURATION, FALL_BLEND_FRACTION, LAND_BLEND_FRACTION,
};
