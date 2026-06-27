use std::marker::PhantomData;

use crozon_rigs::humanoid::LegSegmentLengths;

use crate::animations::{Land, Spring, Squat, vertical_drop};

pub const SQUAT_SEGMENT_END: f32 = 0.20;
pub const SPRING_SEGMENT_END: f32 = 0.40;
pub const FALL_SEGMENT_END: f32 = 0.75;

const JUMP_HEIGHT_MULTIPLIER: f32 = 2.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpSegment {
	Squat,
	Spring,
	Fall,
	Land,
}

#[derive(Debug, Clone)]
pub struct TwoFootedJump<Rig> {
	pub phase: f32,
	pub jump_height: Option<f32>,
	_rig: PhantomData<Rig>,
}

impl<Rig> TwoFootedJump<Rig> {
	pub fn new(phase: f32) -> Self {
		Self { phase, jump_height: None, _rig: PhantomData }
	}

	pub fn from_time(t: f32, cycle_speed: f32) -> Self {
		Self::new((t * cycle_speed).fract())
	}

	pub fn segment_at(phase: f32) -> (JumpSegment, f32) {
		let p = phase.fract();
		if p < SQUAT_SEGMENT_END {
			(JumpSegment::Squat, p / SQUAT_SEGMENT_END)
		} else if p < SPRING_SEGMENT_END {
			(JumpSegment::Spring, (p - SQUAT_SEGMENT_END) / (SPRING_SEGMENT_END - SQUAT_SEGMENT_END))
		} else if p < FALL_SEGMENT_END {
			(JumpSegment::Fall, (p - SPRING_SEGMENT_END) / (FALL_SEGMENT_END - SPRING_SEGMENT_END))
		} else {
			(JumpSegment::Land, (p - FALL_SEGMENT_END) / (1.0 - FALL_SEGMENT_END))
		}
	}

	pub fn segment(&self) -> (JumpSegment, f32) {
		Self::segment_at(self.phase)
	}

	pub fn jump_height(&self, lengths: LegSegmentLengths) -> f32 {
		self.jump_height.unwrap_or_else(|| {
			let squat = Squat::<Rig>::default();
			let max_drop = vertical_drop(squat.femur_peak, squat.shin_peak, lengths);
			max_drop * JUMP_HEIGHT_MULTIPLIER
		})
	}

	pub fn vertical_offset(&self, lengths: LegSegmentLengths) -> f32 {
		let (segment, local) = self.segment();
		let squat = Squat::<Rig>::new(local);
		let jump_height = self.jump_height(lengths);

		match segment {
			JumpSegment::Squat => -squat.vertical_drop(lengths),
			JumpSegment::Spring => {
				let spring = Spring::<Rig>::new(local, Squat::<Rig>::default());
				jump_height * spring.extend_amount()
			}
			JumpSegment::Fall => jump_height,
			JumpSegment::Land => {
				let land = Land::<Rig>::new(local, Squat::<Rig>::default());
				let descent = jump_height * (1.0 - local);
				descent - land.vertical_drop(lengths)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn segment_routing_at_boundaries() -> anyhow::Result<()> {
		let (seg, local) = TwoFootedJump::<()>::segment_at(0.0);
		assert_eq!(seg, JumpSegment::Squat);
		assert!(local.abs() < 1e-5);

		let (seg, local) = TwoFootedJump::<()>::segment_at(SQUAT_SEGMENT_END);
		assert_eq!(seg, JumpSegment::Spring);
		assert!(local.abs() < 1e-5);

		let (seg, local) = TwoFootedJump::<()>::segment_at(SPRING_SEGMENT_END);
		assert_eq!(seg, JumpSegment::Fall);
		assert!(local.abs() < 1e-5);

		let (seg, local) = TwoFootedJump::<()>::segment_at(FALL_SEGMENT_END);
		assert_eq!(seg, JumpSegment::Land);
		assert!(local.abs() < 1e-5);

		Ok(())
	}

	#[test]
	fn vertical_profile_endpoints_and_peak() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::new(0.0);
		assert!(jump.vertical_offset(lengths).abs() < 1e-5);

		let jump = TwoFootedJump::<()>::new(0.99);
		assert!(jump.vertical_offset(lengths).abs() < 0.05);

		let squat_bottom = TwoFootedJump::<()>::new(SQUAT_SEGMENT_END * 0.5);
		assert!(squat_bottom.vertical_offset(lengths) < 0.0);

		let squat_end = TwoFootedJump::<()>::new(SQUAT_SEGMENT_END * 0.99);
		assert!(squat_end.vertical_offset(lengths).abs() < 0.05);

		let apex = TwoFootedJump::<()>::new((SPRING_SEGMENT_END + FALL_SEGMENT_END) * 0.5);
		assert!(apex.vertical_offset(lengths) > 0.0);

		Ok(())
	}
}
