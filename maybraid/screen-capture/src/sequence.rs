//! Reusable capture-sequence runner.
//!
//! Domain code applies a shot and reports readiness. This type owns settle
//! timing, capture-once, and advance. Isolated subjects still go through
//! [`crate::ScreenCapturePlugin`]; complete scenes use [`crate::scene::SceneCapture`].

/// One shot in a [`CaptureSequence`].
pub trait CaptureShot: Clone + Send + Sync + 'static {
	fn settle_secs(&self) -> f32;

	fn hold_after_capture_secs(&self) -> f32 {
		0.08
	}
}

/// What the caller should do after [`CaptureSequence::advance`] or [`CaptureSequence::drive`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureSequenceTick {
	Wait,
	Capture,
	Finished,
}

/// Ordered shots with settle / capture / hold state.
#[derive(Clone, Debug)]
pub struct CaptureSequence<S> {
	shots: Vec<S>,
	index: usize,
	dwell: f32,
	last_applied: Option<usize>,
	captured_this_step: bool,
}

impl<S> CaptureSequence<S> {
	pub fn new(shots: impl Into<Vec<S>>) -> Self {
		Self {
			shots: shots.into(),
			index: 0,
			dwell: 0.0,
			last_applied: None,
			captured_this_step: false,
		}
	}

	pub fn shots(&self) -> &[S] {
		&self.shots
	}

	pub fn index(&self) -> usize {
		self.index
	}

	pub fn current(&self) -> Option<&S> {
		self.shots.get(self.index)
	}

	pub fn is_finished(&self) -> bool {
		self.index >= self.shots.len()
	}

	pub fn needs_apply(&self) -> bool {
		!self.is_finished() && self.last_applied != Some(self.index)
	}

	pub fn mark_applied(&mut self) {
		if !self.is_finished() {
			self.last_applied = Some(self.index);
		}
	}

	pub fn mark_captured(&mut self) {
		self.captured_this_step = true;
		self.dwell = 0.0;
	}
}

impl<S: CaptureShot> CaptureSequence<S> {
	/// Advance settle / hold timing. Call [`Self::mark_applied`] first when
	/// [`Self::needs_apply`] is set, then [`Self::mark_captured`] on [`CaptureSequenceTick::Capture`].
	pub fn advance(&mut self, dt: f32, ready: bool) -> CaptureSequenceTick {
		if self.is_finished() {
			return CaptureSequenceTick::Finished;
		}
		if self.last_applied != Some(self.index) {
			return CaptureSequenceTick::Wait;
		}
		self.dwell += dt;
		if self.captured_this_step {
			if self.dwell < self.shots[self.index].hold_after_capture_secs() {
				return CaptureSequenceTick::Wait;
			}
			self.index += 1;
			self.dwell = 0.0;
			self.captured_this_step = false;
			self.last_applied = None;
			if self.is_finished() {
				return CaptureSequenceTick::Finished;
			}
			return CaptureSequenceTick::Wait;
		}
		if self.dwell < self.shots[self.index].settle_secs() || !ready {
			return CaptureSequenceTick::Wait;
		}
		CaptureSequenceTick::Capture
	}

	/// Apply the current shot through `apply`, then [`Self::advance`].
	pub fn drive(
		&mut self,
		dt: f32,
		ready: bool,
		mut apply: impl FnMut(&S),
	) -> CaptureSequenceTick {
		if self.needs_apply() {
			if let Some(shot) = self.current().cloned() {
				apply(&shot);
				self.mark_applied();
			}
		}
		self.advance(dt, ready)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, Debug, PartialEq, Eq)]
	struct Shot {
		label: &'static str,
		settle: u32,
	}

	impl CaptureShot for Shot {
		fn settle_secs(&self) -> f32 {
			self.settle as f32 * 0.1
		}
	}

	fn two_shots() -> CaptureSequence<Shot> {
		CaptureSequence::new([Shot { label: "a", settle: 2 }, Shot { label: "b", settle: 1 }])
	}

	#[test]
	fn drive_applies_settles_captures_and_finishes() -> anyhow::Result<()> {
		let mut sequence = two_shots();
		let mut applied = Vec::new();
		assert_eq!(
			sequence.drive(0.1, true, |shot| applied.push(shot.label)),
			CaptureSequenceTick::Wait
		);
		assert_eq!(applied, ["a"]);
		assert_eq!(sequence.drive(0.1, true, |_| {}), CaptureSequenceTick::Capture);
		sequence.mark_captured();
		assert_eq!(
			sequence.drive(0.08, true, |shot| applied.push(shot.label)),
			CaptureSequenceTick::Wait
		);
		assert_eq!(applied, ["a"]);
		assert_eq!(
			sequence.drive(0.05, true, |shot| applied.push(shot.label)),
			CaptureSequenceTick::Wait
		);
		assert_eq!(applied, ["a", "b"]);
		assert_eq!(sequence.drive(0.05, true, |_| {}), CaptureSequenceTick::Capture);
		sequence.mark_captured();
		assert_eq!(sequence.drive(0.08, true, |_| {}), CaptureSequenceTick::Finished);
		assert!(sequence.is_finished());
		Ok(())
	}

	#[test]
	fn readiness_false_delays_capture() -> anyhow::Result<()> {
		let mut sequence = two_shots();
		assert_eq!(sequence.drive(0.2, false, |_| {}), CaptureSequenceTick::Wait);
		assert_eq!(sequence.drive(0.0, true, |_| {}), CaptureSequenceTick::Capture);
		Ok(())
	}

	#[test]
	fn empty_sequence_is_finished() -> anyhow::Result<()> {
		let mut sequence = CaptureSequence::<Shot>::new([]);
		assert_eq!(sequence.drive(0.1, true, |_| {}), CaptureSequenceTick::Finished);
		Ok(())
	}
}
