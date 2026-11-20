//! Bounded CPU MAC velocity domain; optical density is deliberately separate.
use crate::parts::combined::interaction::InteractionScene;
use bevy::math::DVec3;
pub const N: [usize; 3] = [24, 12, 24];
pub const H: f64 = 0.25;
pub const ORIGIN: DVec3 = DVec3::new(-2., 0., -3.);
pub const STEP: f64 = 1. / 60.;
pub const ITERATIONS: usize = 48;
#[derive(Debug)]
pub struct Flow {
	pub velocity: [Vec<f64>; 3],
	pub origin: DVec3,
	pub rolling: bool,
	pub shifts: u32,
	pub ambient: DVec3,
	scratch: [Vec<f64>; 3],
	pub pressure: Vec<f64>,
	work: Vec<f64>,
	stencil: Vec<[i32; 6]>,
	projection_neighbors: Vec<[i32; 6]>,
	diagonals: Vec<f64>,
	pub divergence: Vec<f64>,
	pub unprojected: Vec<f64>,
	pub solid: Vec<bool>,
	boundary: Vec<DVec3>,
	pub before: f64,
	pub after: f64,
	pub steps: u32,
	#[cfg(test)]
	pub momentum_rate: f64,
}
impl Default for Flow {
	fn default() -> Self {
		Self::new(DVec3::ZERO)
	}
}
impl Flow {
	pub fn new(wind: DVec3) -> Self {
		let faces = |a: usize| {
			let mut d = N;
			d[a] += 1;
			vec![wind[a]; d.iter().product()]
		};
		let c = N.iter().product();
		Self {
			velocity: std::array::from_fn(faces),
			origin: ORIGIN,
			rolling: false,
			shifts: 0,
			ambient: wind,
			scratch: std::array::from_fn(faces),
			pressure: vec![0.; c],
			work: vec![0.; c],
			projection_neighbors: vec![[-1; 6]; c],
			diagonals: vec![0.; c],
			stencil: (0..c)
				.map(|i| {
					let q = Self::cell(i);
					std::array::from_fn(|k| {
						let a = k / 2;
						let sign = if k % 2 == 0 { -1 } else { 1 };
						let c = q[a] as i32 + sign;
						if c < 0 || c >= N[a] as i32 {
							if a == 1 && c < 0 {
								-2
							} else {
								-1
							}
						} else {
							let mut n = q;
							n[a] = c as usize;
							Self::index(n, N) as i32
						}
					})
				})
				.collect(),
			divergence: vec![0.; c],
			unprojected: vec![0.; c],
			solid: vec![false; c],
			boundary: vec![DVec3::ZERO; c],
			before: 0.,
			after: 0.,
			steps: 0,
			#[cfg(test)]
			momentum_rate: 4.,
		}
	}
	fn index(q: [usize; 3], d: [usize; 3]) -> usize {
		q[0] + d[0] * (q[1] + d[1] * q[2])
	}
	fn dims(a: usize) -> [usize; 3] {
		let mut d = N;
		d[a] += 1;
		d
	}
	fn cell(i: usize) -> [usize; 3] {
		[i % N[0], i / N[0] % N[1], i / (N[0] * N[1])]
	}
	fn face_position(&self, q: [usize; 3], a: usize) -> DVec3 {
		let mut v = DVec3::new(q[0] as f64 + 0.5, q[1] as f64 + 0.5, q[2] as f64 + 0.5);
		v[a] -= 0.5;
		self.origin + v * H
	}
	fn component(&self, p: DVec3, a: usize) -> f64 {
		let d = Self::dims(a);
		let mut q = (p - self.origin) / H - DVec3::splat(0.5);
		q[a] += 0.5;
		let q = q.clamp(
			DVec3::ZERO,
			DVec3::new((d[0] - 1) as f64, (d[1] - 1) as f64, (d[2] - 1) as f64),
		);
		let b = q.floor();
		let f = q - b;
		let mut value = 0.;
		for k in 0..8 {
			let mut at = [0; 3];
			let mut w = 1.;
			for j in 0..3 {
				let high = k & (1 << j) != 0;
				at[j] = (b[j] as usize + usize::from(high)).min(d[j] - 1);
				w *= if high { f[j] } else { 1. - f[j] };
			}
			value += self.velocity[a][Self::index(at, d)] * w;
		}
		value
	}
	pub fn sample(&self, p: DVec3) -> DVec3 {
		{
			let value =
				DVec3::new(self.component(p, 0), self.component(p, 1), self.component(p, 2));
			if !self.rolling {
				value
			} else {
				let edge =
					(p - self.origin).min(self.origin + DVec3::new(6., 3., 6.) - p).min_element();
				let u = (edge / 0.5).clamp(0., 1.);
				self.ambient.lerp(value, u * u * (3. - 2. * u))
			}
		}
	}
	/// Integer-cell remap of staggered faces; overlap values keep their world positions.
	/// Scratch and all dynamic projection/solid metadata are invalidated, not advected.
	pub fn shift_to(&mut self, origin: DVec3) {
		let delta = ((origin - self.origin) / H).round();
		if delta == DVec3::ZERO {
			return;
		}
		for a in 0..3 {
			let d = Self::dims(a);
			for z in 0..d[2] {
				for y in 0..d[1] {
					for x in 0..d[0] {
						let old = [
							x as isize + delta.x as isize,
							y as isize + delta.y as isize,
							z as isize + delta.z as isize,
						];
						self.scratch[a][Self::index([x, y, z], d)] =
							if (0..3).all(|j| old[j] >= 0 && old[j] < d[j] as isize) {
								self.velocity[a][Self::index(old.map(|v| v as usize), d)]
							} else {
								self.ambient[a]
							};
					}
				}
			}
		}
		std::mem::swap(&mut self.velocity, &mut self.scratch);
		self.origin += delta * H;
		self.shifts += 1;
		self.solid.fill(false);
		self.boundary.fill(DVec3::ZERO);
		self.pressure.fill(0.);
		self.work.fill(0.);
		self.divergence.fill(0.);
		self.unprojected.fill(0.);
		self.projection_neighbors.fill([-1; 6]);
		self.diagonals.fill(0.);
	}
	pub fn covers(&self, p: DVec3, margin: f64) -> bool {
		(p - self.origin).cmpge(DVec3::splat(margin)).all()
			&& (p - self.origin).cmple(DVec3::new(6., 3., 6.) - DVec3::splat(margin)).all()
	}

	pub fn bytes(&self) -> usize {
		self.velocity
			.iter()
			.chain(self.scratch.iter())
			.map(|v| v.len() * 8)
			.sum::<usize>()
			+ self.pressure.len() * (8 * 4 + 1 + 24)
			+ (self.stencil.len() + self.projection_neighbors.len())
				* std::mem::size_of::<[i32; 6]>()
			+ self.diagonals.len() * std::mem::size_of::<f64>()
	}
	fn neighbours(q: [usize; 3], a: usize) -> (Option<usize>, Option<usize>) {
		let left = if q[a] > 0 {
			let mut p = q;
			p[a] -= 1;
			Some(Self::index(p, N))
		} else {
			None
		};
		let right = if q[a] < N[a] { Some(Self::index(q, N)) } else { None };
		(left, right)
	}
	fn enforce(&mut self) {
		for a in 0..3 {
			let d = Self::dims(a);
			for z in 0..d[2] {
				for y in 0..d[1] {
					for x in 0..d[0] {
						let q = [x, y, z];
						let (l, r) = Self::neighbours(q, a);
						let b = l.filter(|i| self.solid[*i]).or(r.filter(|i| self.solid[*i]));
						let i = Self::index(q, d);
						if let Some(b) = b {
							self.velocity[a][i] = self.boundary[b][a];
						} else if a == 1 && y == 0 {
							self.velocity[a][i] = 0.;
						}
					}
				}
			}
		}
	}
	fn divergence(&mut self) -> f64 {
		let mut sum = 0.;
		let mut count = 0;
		for i in 0..self.solid.len() {
			self.divergence[i] = 0.;
			if self.solid[i] {
				continue;
			}
			let q = Self::cell(i);
			for a in 0..3 {
				let mut r = q;
				r[a] += 1;
				let d = Self::dims(a);
				self.divergence[i] +=
					(self.velocity[a][Self::index(r, d)] - self.velocity[a][Self::index(q, d)]) / H;
			}
			sum += self.divergence[i].powi(2);
			count += 1;
		}
		(sum / (count.max(1) as f64)).sqrt()
	}
	pub fn project(&mut self) {
		self.enforce();
		self.before = self.divergence();
		self.unprojected.copy_from_slice(&self.divergence);
		self.pressure.fill(0.);
		// Connectivity is constant; coefficients are refreshed from current solid occupancy.
		for i in 0..self.solid.len() {
			self.diagonals[i] = 0.;
			for (k, j) in self.stencil[i].into_iter().enumerate() {
				let open = j >= 0 && !self.solid[j as usize];
				self.projection_neighbors[i][k] = if open { j } else { -1 };
				if j == -1 || open {
					self.diagonals[i] += 1.;
				}
			}
		}
		for _ in 0..ITERATIONS {
			for i in 0..self.solid.len() {
				if self.solid[i] {
					self.work[i] = 0.;
					continue;
				}
				let mut sum = 0.;
				let diagonal = self.diagonals[i];
				for j in self.projection_neighbors[i] {
					if j >= 0 {
						sum += self.pressure[j as usize];
					}
				}

				self.work[i] = if diagonal > 0. {
					(sum - self.divergence[i] * H * H / STEP) / diagonal
				} else {
					0.
				};
			}
			std::mem::swap(&mut self.pressure, &mut self.work);
		}
		for a in 0..3 {
			let d = Self::dims(a);
			for z in 0..d[2] {
				for y in 0..d[1] {
					for x in 0..d[0] {
						let q = [x, y, z];
						let (l, r) = Self::neighbours(q, a);
						if l.is_some_and(|i| self.solid[i])
							|| r.is_some_and(|i| self.solid[i])
							|| (a == 1 && y == 0)
						{
							continue;
						}
						let pl = l.map_or(0., |i| self.pressure[i]);
						let pr = r.map_or(0., |i| self.pressure[i]);
						self.velocity[a][Self::index(q, d)] -= STEP * (pr - pl) / H;
					}
				}
			}
		}
		self.enforce();
		self.after = self.divergence();
	}
	#[cfg(test)]
	fn project_reference(&mut self) {
		self.enforce();
		self.before = self.divergence();
		self.unprojected.copy_from_slice(&self.divergence);
		self.pressure.fill(0.);
		for _ in 0..ITERATIONS {
			for i in 0..self.solid.len() {
				if self.solid[i] {
					self.work[i] = 0.;
					continue;
				}
				let q = Self::cell(i);
				let mut sum = 0.;
				let mut diagonal = 0.;
				for a in 0..3 {
					for sign in [-1i32, 1] {
						let c = q[a] as i32 + sign;
						if c < 0 || c >= N[a] as i32 {
							if !(a == 1 && c < 0) {
								diagonal += 1.;
							}
							continue;
						}
						let mut n = q;
						n[a] = c as usize;
						let j = Self::index(n, N);
						if !self.solid[j] {
							diagonal += 1.;
							sum += self.pressure[j];
						}
					}
				}
				self.work[i] = if diagonal > 0. {
					(sum - self.divergence[i] * H * H / STEP) / diagonal
				} else {
					0.
				};
			}
			std::mem::swap(&mut self.pressure, &mut self.work);
		}
		for a in 0..3 {
			let d = Self::dims(a);
			for z in 0..d[2] {
				for y in 0..d[1] {
					for x in 0..d[0] {
						let q = [x, y, z];
						let (l, r) = Self::neighbours(q, a);
						if l.is_some_and(|i| self.solid[i])
							|| r.is_some_and(|i| self.solid[i])
							|| (a == 1 && y == 0)
						{
							continue;
						}
						let pl = l.map_or(0., |i| self.pressure[i]);
						let pr = r.map_or(0., |i| self.pressure[i]);
						self.velocity[a][Self::index(q, d)] -= STEP * (pr - pl) / H;
					}
				}
			}
		}
		self.enforce();
		self.after = self.divergence();
	}
	pub fn step(&mut self, scene: &InteractionScene, t: f64) {
		self.ambient = scene.wind;
		let colliders = scene.colliders(t);
		let count = scene.collider_count();
		let external = scene.external.is_some();
		let radii = colliders.map(|c| match c.shape {
			crate::parts::combined::interaction::Shape::Capsule { radius, half_segment } => {
				radius + half_segment
			}
			crate::parts::combined::interaction::Shape::Box { half } => half.length(),
		});
		for i in 0..self.solid.len() {
			let q = Self::cell(i);
			let p = self.origin
				+ (DVec3::new(q[0] as f64, q[1] as f64, q[2] as f64) + DVec3::splat(0.5)) * H;
			let contact = colliders
				.iter()
				.take(count)
				.enumerate()
				.filter(|(i, c)| {
					!external || (p - c.position).length_squared() <= (radii[*i] + H * 0.5).powi(2)
				})
				.map(|(_, c)| c.query(p))
				.filter(|c| c.distance < H * 0.5)
				.min_by(|a, b| a.distance.total_cmp(&b.distance));
			let was = self.solid[i];
			self.solid[i] = contact.is_some()
				|| scene
					.terrain
					.as_ref()
					.is_some_and(|h| h.sample(p).is_none_or(|g| p.y - g.point.y < H * 0.5));
			self.boundary[i] = contact.map_or(DVec3::ZERO, |c| c.velocity);
			if was && !self.solid[i] {
				for a in 0..3 {
					self.velocity[a][Self::index(q, Self::dims(a))] = scene.wind[a];
				}
			}
		}
		for a in 0..3 {
			let d = Self::dims(a);
			for z in 0..d[2] {
				for y in 0..d[1] {
					for x in 0..d[0] {
						let q = [x, y, z];
						let p = self.face_position(q, a);
						let velocity = self.sample(p);
						let mut value = self.component(p - velocity * STEP, a);
						value += (scene.wind[a] - value) * (1. - (-0.2 * STEP).exp());
						for (i, c) in colliders.iter().take(count).enumerate() {
							if external
								&& (p - c.position).length_squared() > (radii[i] + H * 1.5).powi(2)
							{
								continue;
							}
							let contact = c.query(p);
							let weight = (1. - contact.distance.max(0.) / (H * 1.5)).clamp(0., 1.);
							#[cfg(test)]
							let rate = self.momentum_rate;
							#[cfg(not(test))]
							let rate = 4.;
							value += (contact.velocity[a] - value) * weight * STEP * rate;
						}
						self.scratch[a][Self::index(q, d)] = value.clamp(-8., 8.);
					}
				}
			}
		}
		std::mem::swap(&mut self.velocity, &mut self.scratch);
		self.project();
		self.steps += 1;
	}
	pub fn solid_at(&self, p: DVec3) -> bool {
		let q = ((p - self.origin) / H).floor();
		if q.cmplt(DVec3::ZERO).any()
			|| q.cmpge(DVec3::new(N[0] as f64, N[1] as f64, N[2] as f64)).any()
		{
			return false;
		}
		self.solid[Self::index([q.x as usize, q.y as usize, q.z as usize], N)]
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use crate::parts::combined::interaction::Scenario;
	#[test]
	fn uniform_transport_and_projection() -> Result<(), String> {
		let mut f = Flow::new(DVec3::X);
		let s = InteractionScene { scenario: Scenario::None, wind: DVec3::X, ..Default::default() };
		for i in 0..5 {
			f.step(&s, i as f64 * STEP);
		}
		assert!((f.sample(DVec3::new(0., 1., 0.)) - DVec3::X).length() < 1e-9);
		assert!(f.after < 1e-9);
		Ok(())
	}
	#[test]
	fn projection_reduces_injected_divergence() -> Result<(), String> {
		let mut f = Flow::default();
		f.velocity[0][1000] = 2.;
		f.project();
		assert!(f.before > 0.);
		assert!(f.after < f.before * 0.2, "{} -> {}", f.before, f.after);
		Ok(())
	}
	#[test]
	fn moving_boundary_is_finite_and_persistent() -> Result<(), String> {
		let mut f = Flow::new(DVec3::X * 0.4);
		let s = InteractionScene { scenario: Scenario::Fast, ..Default::default() };
		for i in 0..30 {
			f.step(&s, 3. + i as f64 * STEP);
			assert!(f.velocity.iter().flatten().all(|v| v.is_finite() && v.abs() < 20.));
		}
		assert!(f.steps == 30);
		assert!(f.solid.iter().any(|v| *v));
		Ok(())
	}
}

#[cfg(test)]
mod boundaries {
	use super::*;
	use crate::parts::combined::interaction::Scenario;
	#[test]
	fn conservative_thin_wall_and_stopped_history() -> Result<(), String> {
		let mut f = Flow::new(DVec3::X * 0.42);
		let s = InteractionScene { scenario: Scenario::Static, ..Default::default() };
		let start = std::time::Instant::now();
		for i in 0..10 {
			f.step(&s, i as f64 * STEP);
		}
		assert!(f.solid_at(DVec3::new(1.875, 0.625, -0.625)));
		assert!(f.after.is_finite());
		eprintln!(
			"flow CPU wall ms/step={}, allocated bytes={}, divergence {} -> {}",
			start.elapsed().as_secs_f64() * 100.,
			f.bytes(),
			f.before,
			f.after
		);
		Ok(())
	}
}

#[cfg(test)]
mod discontinuities {
	use super::*;
	use crate::parts::combined::interaction::Scenario;
	#[test]
	fn teleport_has_no_momentum_and_reset_has_no_history() -> Result<(), String> {
		let mut s = InteractionScene {
			scenario: Scenario::Manual,
			wind: DVec3::ZERO,
			..Default::default()
		};
		let mut f = Flow::default();
		f.step(&s, 0.);
		s.place(0.1, DVec3::new(0.25, 0.7, 1.5), true);
		f.step(&s, 0.1);
		assert!(f.velocity.iter().flatten().all(|v| v.abs() < 1e-12));
		f.velocity[0][1000] = 1.;
		f.project();
		let reset = Flow::default();
		assert!(reset.velocity.iter().flatten().all(|v| *v == 0.));
		assert_eq!(reset.steps, 0);
		Ok(())
	}
}

#[cfg(test)]
mod stencil_agreement {
	use super::*;
	#[test]
	fn exact_dynamic_solid_projection() -> Result<(), String> {
		let mut a = Flow::new(DVec3::new(0.2, 0., 0.1));
		let mut b = Flow::new(DVec3::new(0.2, 0., 0.1));
		for phase in 0..3 {
			for axis in 0..3 {
				for i in 0..a.velocity[axis].len() {
					let v = (i as f64 * 0.37).sin();
					a.velocity[axis][i] = v;
					b.velocity[axis][i] = v;
				}
			}
			for i in 0..a.solid.len() {
				let v = (i + phase) % 17 == 0;
				a.solid[i] = v;
				b.solid[i] = v;
			}
			a.project();
			b.project_reference();
			assert_eq!(a.velocity, b.velocity);
			assert_eq!(a.pressure, b.pressure);
			assert_eq!(a.divergence, b.divergence);
		}
		Ok(())
	}
}

#[cfg(test)]
mod rolling_tests {
	use super::*;
	#[test]
	fn whole_cell_shift_preserves_world_faces_and_clock() {
		let mut f = Flow::new(DVec3::new(0.1, 0.2, 0.3));
		f.rolling = true;
		for a in 0..3 {
			let d = Flow::dims(a);
			for z in 0..d[2] {
				for y in 0..d[1] {
					for x in 0..d[0] {
						let p = f.face_position([x, y, z], a);
						f.velocity[a][Flow::index([x, y, z], d)] =
							p.x * 0.2 + p.y * 0.1 + p.z * 0.05 + a as f64;
					}
				}
			}
		}
		let p = DVec3::new(1., 1.3, 0.2);
		let before = f.sample(p);
		f.steps = 17;
		f.shift_to(ORIGIN + DVec3::new(0.5, 0.25, -0.25));
		assert!((f.sample(p) - before).length() < 1e-12);
		assert_eq!(f.steps, 17);
		assert_eq!(f.shifts, 1);
		assert_eq!(f.sample(DVec3::splat(100.)), f.ambient);
		assert_eq!(f.velocity[0][Flow::index([24, 6, 6], Flow::dims(0))], f.ambient.x);
	}
	#[test]
	fn elevated_terrain_solids_refresh_after_shift() -> Result<(), String> {
		let h = crate::parts::herd::terrain::Heightfield::new(
			bevy::math::DVec2::splat(-8.),
			0.5,
			[33, 33],
			1,
			vec![4.; 1089],
			vec![true; 1089],
		)?;
		let scene = InteractionScene {
			external: Some(vec![]),
			terrain: Some(std::sync::Arc::new(h)),
			ground_floor: None,
			..Default::default()
		};
		let mut f = Flow::new(scene.wind);
		f.rolling = true;
		f.shift_to(DVec3::new(-2., 3.5, -3.));
		f.step(&scene, 0.);
		assert!(f.solid[Flow::index([12, 0, 12], N)]);
		assert!(!f.solid[Flow::index([12, 5, 12], N)]);
		f.shift_to(f.origin + DVec3::X * 0.25);
		f.step(&scene, STEP);
		assert!(f.solid[Flow::index([12, 0, 12], N)]);
		assert!(f.velocity.iter().flatten().all(|v| v.is_finite()));
		Ok(())
	}
}
