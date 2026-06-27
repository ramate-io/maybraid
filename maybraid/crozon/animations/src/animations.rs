pub mod fall;
pub mod land;
pub mod mix;
pub mod run;
pub mod spring;
pub mod squat;
pub mod two_footed_jump;

pub use fall::Fall;
pub use land::Land;
pub use mix::{Mix, Smooth, smoothstep};
pub use run::Run;
pub use spring::Spring;
pub use squat::{Squat, descent_ascent_amount, vertical_drop};
pub use two_footed_jump::{
	JumpSegment, JumpTiming, TwoFootedJump, DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT,
	DEFAULT_LAND_RECOVERY_SPEED, DEFAULT_SPRING_DURATION, DEFAULT_SQUAT_DESCENT_SPEED,
	FALL_BLEND_FRACTION, air_duration, ballistic_height, launch_speed,
};
