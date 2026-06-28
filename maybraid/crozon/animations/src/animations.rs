pub mod fall;
pub mod fixed;
pub mod fixed_tuck;
pub mod land;
pub mod mix;
pub mod run;
pub mod spring;
pub mod squat;
pub mod transition;
pub mod tuck;
pub mod tucked_flip;
pub mod two_footed_jump;
pub mod upright_run;
pub mod upright_walk;
pub mod walk;

pub use fall::Fall;
pub use fixed::FixedPosition;
pub use fixed_tuck::FixedTuck;
pub use land::Land;
pub use mix::{smoothstep, Mix, Smooth};
pub use run::Run;
pub use spring::Spring;
pub use squat::{Squat, vertical_drop};
pub use transition::{BlendCurve, Transition, TransitionCurve};
pub use tuck::{Tuck, TuckProfile};
pub use tucked_flip::{FlipDirection, TuckedFlip};
pub use upright_run::UprightRun;
pub use upright_walk::UprightWalk;
pub use walk::Walk;
pub use two_footed_jump::{
	air_duration, ballistic_height, launch_speed, touchdown_time_since_launch, JumpSegment, JumpTiming, TwoFootedJump,
	DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED,
	DEFAULT_SPRING_DURATION, FALL_BLEND_FRACTION, LAND_BLEND_FRACTION, LAND_POSE_BLEND_MAX_SECS,
};
