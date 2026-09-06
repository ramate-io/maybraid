//! Middle-out greedy guillotine: place absolute steps outward from each axis mid.

use crate::guillotine::bounds::Bounds;
use crate::guillotine::config::GuillotineConfig;
use crate::guillotine::regions::GuillotineCuts;
use crate::noise::config::NoiseConfig;
use noise::{NoiseFn, Seedable};

/// Deterministic guillotine cutter with a fixed cut-attempt depth.
///
/// This is a **max-fitting / greedy packing** pass, not a perfect tiler:
/// `[step_min, step_max]` is a preferred size window for successive cuts. Terminal
/// remainders at the ends of each axis are unconstrained and may be smaller than
/// `step_min` (arbitrary roots + windows cannot always be partitioned so every leaf
/// lies in the window).
///
/// For each attempt in `0..depth`:
/// 1. Choose an axis that is not yet fully saturated (both outward fronts stuck).
/// 2. Choose an unsaturated side (low / high) on that axis.
/// 3. Sample a step in `[step_min, step_max]` and place the next cut **outward** from
///    the axis midpoint: low front moves toward `min`, high front toward `max`.
/// 4. If the candidate would leave the root extent, discard it and saturate that side.
///
/// Leaf regions are the cartesian product of per-axis intervals (through-cuts of the root).
#[derive(Debug, Clone, PartialEq)]
pub struct Guillotine<const D: usize, N: NoiseFn<f64, D> + Seedable> {
	noise: NoiseConfig<D, N>,
	config: GuillotineConfig,
	depth: u8,
}

#[derive(Clone, Copy)]
struct AxisFront {
	/// Outward cursor on the low side (starts at axis mid; decreases).
	lo: f32,
	/// Outward cursor on the high side (starts at axis mid; increases).
	hi: f32,
	lo_saturated: bool,
	hi_saturated: bool,
}

impl AxisFront {
	fn new(mid: f32) -> Self {
		Self { lo: mid, hi: mid, lo_saturated: false, hi_saturated: false }
	}

	fn saturated(&self) -> bool {
		self.lo_saturated && self.hi_saturated
	}
}

impl<const D: usize, N: NoiseFn<f64, D> + Seedable> Guillotine<D, N> {
	pub fn new(noise: NoiseConfig<D, N>, config: GuillotineConfig, depth: u8) -> Self {
		Self { noise, config, depth }
	}

	pub fn with_noise(noise: NoiseConfig<D, N>) -> Self {
		Self::new(noise, GuillotineConfig::default(), 4)
	}

	pub fn with_depth(mut self, depth: u8) -> Self {
		self.depth = depth;
		self
	}

	pub fn with_config(mut self, config: GuillotineConfig) -> Self {
		self.config = config;
		self
	}

	pub fn depth(&self) -> u8 {
		self.depth
	}

	pub fn set_depth(&mut self, depth: u8) {
		self.depth = depth;
	}

	pub fn config(&self) -> &GuillotineConfig {
		&self.config
	}

	pub fn config_mut(&mut self) -> &mut GuillotineConfig {
		&mut self.config
	}

	pub fn set_config(&mut self, config: GuillotineConfig) {
		self.config = config;
	}

	pub fn noise(&self) -> &NoiseConfig<D, N> {
		&self.noise
	}

	pub fn noise_mut(&mut self) -> &mut NoiseConfig<D, N> {
		&mut self.noise
	}

	pub fn set_noise(&mut self, noise: NoiseConfig<D, N>) {
		self.noise = noise;
	}

	pub fn with_seed(mut self, seed: u32) -> Self {
		self.noise = self.noise.with_seed(seed);
		self
	}

	pub fn with_frequency(mut self, frequency: f32) -> Self {
		self.noise = self.noise.with_frequency(frequency);
		self
	}

	/// Run the middle-out cut pass and return root + per-axis interior cuts.
	pub fn cut(&self, root: Bounds<D>) -> GuillotineCuts<D> {
		let mut cuts: [Vec<f32>; D] = std::array::from_fn(|_| Vec::new());
		let mut fronts: [AxisFront; D] = std::array::from_fn(|axis| {
			let mid = 0.5 * (root.min[axis] + root.max[axis]);
			AxisFront::new(mid)
		});

		let anchor = root.lower_left();
		for attempt in 0..self.depth {
			let Some(axis) = self.choose_axis(&fronts, anchor, attempt) else {
				break;
			};

			let toward_low = self.choose_side(&fronts[axis], anchor, attempt, axis);
			let step = self.noise.sample_range_f32(
				self.config.step_min,
				self.config.step_max,
				noise_sample_point(anchor, attempt, SALT_STEP + axis as f32 * SALT_AXIS_STRIDE),
			);

			let front = &mut fronts[axis];
			let (candidate, from) =
				if toward_low { (front.lo - step, front.lo) } else { (front.hi + step, front.hi) };

			let mut next = candidate;
			if let Some(q) = self.config.snap_quantum {
				if q > 0.0 {
					let origin = root.min[axis];
					let steps = ((next - origin) / q).round();
					next = origin + steps * q;
				}
			}

			// Must move strictly outward from the current front and stay inside the root.
			let valid = next.is_finite()
				&& next > root.min[axis]
				&& next < root.max[axis]
				&& if toward_low { next < from } else { next > from };

			if !valid {
				if toward_low {
					front.lo_saturated = true;
				} else {
					front.hi_saturated = true;
				}
				continue;
			}

			cuts[axis].push(next);
			if toward_low {
				front.lo = next;
			} else {
				front.hi = next;
			}
		}

		for axis in 0..D {
			cuts[axis].sort_by(|a, b| a.total_cmp(b));
			cuts[axis].dedup_by(|a, b| (*a - *b).abs() <= f32::EPSILON);
		}

		GuillotineCuts::new(root, cuts)
	}

	/// Cut `root` and iterate the resulting leaf regions.
	pub fn regions(&self, root: Bounds<D>) -> RegionsOwned<D> {
		RegionsOwned { cuts: self.cut(root) }
	}

	/// Cut `root` and collect leaf regions.
	pub fn regions_vec(&self, root: Bounds<D>) -> Vec<Bounds<D>> {
		self.cut(root).regions_vec()
	}

	fn choose_axis(&self, fronts: &[AxisFront; D], anchor: [f32; D], attempt: u8) -> Option<usize> {
		let mut candidates = [0usize; D];
		let mut n = 0usize;
		for axis in 0..D {
			if !fronts[axis].saturated() {
				candidates[n] = axis;
				n += 1;
			}
		}
		if n == 0 {
			return None;
		}
		let idx =
			self.noise
				.sample_range_usize(0, n, noise_sample_point(anchor, attempt, SALT_AXIS));
		Some(candidates[idx])
	}

	/// Pick low vs high front among unsaturated sides (`true` = toward `min`).
	fn choose_side(&self, front: &AxisFront, anchor: [f32; D], attempt: u8, axis: usize) -> bool {
		match (front.lo_saturated, front.hi_saturated) {
			(false, false) => {
				self.noise.sample_range_usize(
					0,
					2,
					noise_sample_point(anchor, attempt, SALT_SIDE + axis as f32 * SALT_AXIS_STRIDE),
				) == 0
			}
			(false, true) => true,
			(true, false) => false,
			(true, true) => true, // unreachable: axis would already be skipped
		}
	}
}

/// Owning region iterator produced by [`Guillotine::regions`].
#[derive(Debug, Clone)]
pub struct RegionsOwned<const D: usize> {
	cuts: GuillotineCuts<D>,
}

impl<const D: usize> RegionsOwned<D> {
	pub fn cuts(&self) -> &GuillotineCuts<D> {
		&self.cuts
	}
}

impl<const D: usize> IntoIterator for RegionsOwned<D> {
	type Item = Bounds<D>;
	type IntoIter = RegionsIntoIter<D>;

	fn into_iter(self) -> Self::IntoIter {
		let counts = std::array::from_fn(|i| self.cuts.cuts[i].len() + 1);
		let finished = counts.iter().any(|&c| c == 0);
		RegionsIntoIter { cuts: self.cuts, counts, cursor: [0; D], finished }
	}
}

/// Owning odometer iterator over leaf regions.
#[derive(Debug, Clone)]
pub struct RegionsIntoIter<const D: usize> {
	cuts: GuillotineCuts<D>,
	counts: [usize; D],
	cursor: [usize; D],
	finished: bool,
}

impl<const D: usize> Iterator for RegionsIntoIter<D> {
	type Item = Bounds<D>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.finished {
			return None;
		}
		let mut min = [0.0; D];
		let mut max = [0.0; D];
		for axis in 0..D {
			let i = self.cursor[axis];
			min[axis] = if i == 0 { self.cuts.root.min[axis] } else { self.cuts.cuts[axis][i - 1] };
			max[axis] = if i + 1 >= self.counts[axis] {
				self.cuts.root.max[axis]
			} else {
				self.cuts.cuts[axis][i]
			};
		}
		for axis in 0..D {
			self.cursor[axis] += 1;
			if self.cursor[axis] < self.counts[axis] {
				return Some(Bounds::new(min, max));
			}
			self.cursor[axis] = 0;
		}
		self.finished = true;
		Some(Bounds::new(min, max))
	}
}

/// Build a noise **query coordinate** from the root lower-left plus attempt / channel salts.
///
/// This is **not** part of the geometric cut rule. Geometry is always
/// `front ± sample_range_f32(step_min, step_max, …)`. `NoiseConfig` samples a field at a
/// point, so without offsets every draw at a fixed root would be highly correlated (or
/// identical) across channels (axis vs step vs side) and across attempts. The salts here
/// only decorrelate those draws—same role as a `w` lane or extra seed channel.
fn noise_sample_point<const D: usize>(anchor: [f32; D], attempt: u8, salt: f32) -> [f32; D] {
	let mut p = anchor;
	p[0] += salt + attempt as f32 * ATTEMPT_STRIDE;
	if D > 1 {
		p[1] += salt * 0.37;
	}
	p
}

const SALT_AXIS: f32 = 31.7;
const SALT_STEP: f32 = -47.3;
const SALT_SIDE: f32 = 59.1;
const SALT_AXIS_STRIDE: f32 = 13.1;
const ATTEMPT_STRIDE: f32 = 101.0;
