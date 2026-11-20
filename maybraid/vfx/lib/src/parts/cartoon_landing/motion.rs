//! Portable landing_motion.py defaults, evaluated analytically in event time.
use bevy::prelude::*;
#[derive(Clone, Debug)]
pub struct Puff {
	pub id: u32,
	pub seed: u32,
	pub variant: usize,
	pub birth: f64,
	pub lifetime: f64,
	angle: f64,
	yaw: f64,
	yaw_change: f64,
	angle_change: f64,
	start: f64,
	end: f64,
	scale: [f64; 3],
	dissolve: f64,
	curl: f64,
	central: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct State {
	pub transform: Transform,
	pub age: f64,
	pub threshold: f32,
}
fn smooth(v: f64) -> f64 {
	let v = v.clamp(0., 1.);
	v * v * (3. - 2. * v)
}
pub fn mix32(mut x: u32) -> u32 {
	x ^= x >> 16;
	x = x.wrapping_mul(0x7feb352d);
	x ^= x >> 15;
	x = x.wrapping_mul(0x846ca68b);
	x ^= x >> 16;
	x
}
pub fn random(seed: u32, id: u32, channel: u32) -> f64 {
	mix32(seed ^ (id + 1).wrapping_mul(0x9e3779b9) ^ (channel + 1).wrapping_mul(0x85ebca6b)) as f64
		/ 4294967296.
}
impl Puff {
	pub const DURATION: f64 = 1.05;
	pub fn table(seed: u32) -> Vec<Self> {
		let r = |i, k| random(seed, i, k);
		let d = Self::DURATION;
		let tau = std::f64::consts::TAU;
		let pi = std::f64::consts::PI;
		let mut out = Vec::with_capacity(16);
		for i in 0..2 {
			out.push(Self {
				id: i,
				seed: mix32(seed ^ (i + 1).wrapping_mul(0x27d4eb2d)),
				variant: ((i + (3. * r(i, 2)) as u32) % 3) as usize,
				birth: i as f64 * 0.018 * d,
				lifetime: d * (0.255 + 0.055 * r(i, 3)),
				angle: tau * r(i, 4),
				yaw: tau * r(i, 5),
				yaw_change: 0.,
				angle_change: 0.03 * (2. * r(i, 6) - 1.),
				start: 0.025 * r(i, 7),
				end: 0.075 + 0.025 * r(i, 8),
				scale: [0.095, 0.071, 0.092],
				dissolve: 0.31 + 0.05 * r(i, 9),
				curl: 0.,
				central: true,
			});
		}
		let weights: Vec<_> = (0..12).map(|i| 0.70 + 0.60 * r(i, 0)).collect();
		let sum: f64 = weights.iter().sum();
		let mut angle = tau * r(91, 1);
		for j in 0..12u32 {
			let i = j + 2;
			let birth = d * (0.027 + 0.108 * r(i, 1));
			let size = 1. + 0.22 * (2. * r(i, 3) - 1.);
			out.push(Self {
				id: i,
				seed: mix32(seed ^ (i + 1).wrapping_mul(0x27d4eb2d)),
				variant: ((j + (2. * r(i, 4)) as u32) % 3) as usize,
				birth,
				lifetime: (d * (0.735 + 0.175 * r(i, 2))).min(d - birth),
				angle,
				yaw: angle + pi * (0.38 + 0.22 * r(i, 5)),
				yaw_change: 0.075 * (2. * r(i, 6) - 1.),
				angle_change: 0.055 * (2. * r(i, 7) - 1.),
				start: 0.105 + 0.065 * r(i, 8),
				end: 1.25 * (0.62 + 0.11 * r(i, 9)),
				scale: [0.292 * size, 0.146 * (0.86 + 0.24 * r(i, 10)), 0.275 * size],
				dissolve: 0.57 + 0.09 * r(i, 11),
				curl: 0.008 + 0.008 * r(i, 12),
				central: false,
			});
			angle += tau * weights[j as usize] / sum;
		}
		let first = (r(80, 0) * 12.) as usize;
		for (k, j) in [first, (first + 7) % 12].into_iter().enumerate() {
			let p = &out[j + 2];
			let i = 14 + k as u32;
			out.push(Self {
				id: i,
				seed: mix32(seed ^ (i + 1).wrapping_mul(0x27d4eb2d)),
				variant: (p.variant + 1) % 3,
				birth: d * (0.143 + 0.036 * r(i, 1)),
				lifetime: d * (0.64 + 0.08 * r(i, 2)),
				angle: p.angle + 0.075 * (2. * r(i, 3) - 1.),
				yaw: p.yaw + 0.20,
				yaw_change: 0.095 * (2. * r(i, 4) - 1.),
				angle_change: 0.06 * (2. * r(i, 5) - 1.),
				start: 0.36 + 0.03 * r(i, 6),
				end: p.end + 0.025,
				scale: p.scale.map(|v| v * 0.72),
				dissolve: 0.56 + 0.055 * r(i, 7),
				curl: 0.025 + 0.010 * r(i, 8),
				central: false,
			});
		}
		out.sort_by(|a, b| a.birth.total_cmp(&b.birth).then(a.id.cmp(&b.id)));
		out
	}
	pub fn state(&self, time: f64) -> Option<State> {
		let age = time - self.birth;
		if age < 0. || age >= self.lifetime {
			return None;
		}
		let a = age / self.lifetime;
		let u = (a / 0.84).clamp(0., 1.);
		let radius = self.start + (self.end - self.start) * (1. - (1. - u).powi(3));
		let theta = self.angle + self.angle_change * smooth(a / 0.85);
		let growth = 0.035 + 0.965 * smooth(a / if self.central { 0.29 } else { 0.245 });
		let mut scale = self.scale.map(|v| v * growth);
		scale[1] *= (0.88 + 0.12 * smooth((a - 0.08) / 0.52))
			* (1. + self.curl * smooth((a - 0.12) / 0.55) / 0.15);
		let yaw = self.yaw + self.yaw_change * smooth(a / 0.75);
		Some(State {
			transform: Transform {
				translation: Vec3::new(
					(theta.cos() * radius) as f32,
					0.004,
					(theta.sin() * radius) as f32,
				),
				rotation: Quat::from_rotation_y(-yaw as f32),
				scale: Vec3::from_array(scale.map(|v| v as f32)),
			},
			age,
			threshold: (-0.02 + 1.04 * smooth((a - self.dissolve) / (0.96 - self.dissolve))) as f32,
		})
	}
}
#[cfg(test)]
mod tests {
	use crate::parts::cartoon_landing::motion::*;
	#[test]
	fn delivered_states_and_seeds() -> Result<(), Box<dyn std::error::Error>> {
		let cases: serde_json::Value = serde_json::from_str(include_str!("reference.json"))?;
		for case in cases.as_array().ok_or("cases")? {
			let seed = case["seed"].as_u64().ok_or("seed")? as u32;
			let time = case["time"].as_f64().ok_or("time")?;
			let table = Puff::table(seed);
			let expected = case["states"].as_array().ok_or("states")?;
			assert_eq!(table.iter().filter_map(|p| p.state(time)).count(), expected.len());
			for e in expected {
				let p =
					table.iter().find(|p| Some(p.id as u64) == e["id"].as_u64()).ok_or("puff")?;
				assert_eq!(Some(p.seed as u64), e["seed"].as_u64());
				assert_eq!(Some(p.variant as u64), e["variant"].as_u64());
				let s = p.state(time).ok_or("missing live state")?;
				for k in 0..3 {
					assert!(
						(s.transform.translation[k] as f64
							- e["position"][k].as_f64().ok_or("position")?)
						.abs() < 1e-6
					);
					assert!(
						(s.transform.scale[k] as f64 - e["scale"][k].as_f64().ok_or("scale")?)
							.abs() < 1e-6
					);
				}
				assert!(
					(s.threshold as f64 - e["dissolve_threshold"].as_f64().ok_or("threshold")?)
						.abs() < 1e-6
				);
				assert!(s.transform.scale.min_element() > 0.);
				let rest = Vec3::new(0.2, 0.7, -0.3);
				let yaw = e["yaw"].as_f64().ok_or("yaw")?;
				let scaled = rest * s.transform.scale;
				let expected = Vec3::new(
					(yaw.cos() * scaled.x as f64 - yaw.sin() * scaled.z as f64) as f32,
					scaled.y,
					(yaw.sin() * scaled.x as f64 + yaw.cos() * scaled.z as f64) as f32,
				) + s.transform.translation;
				assert!((s.transform.transform_point(rest) - expected).length() < 1e-6);
			}
		}
		Ok(())
	}
	#[test]
	fn birth_expiry_and_partition_independence() {
		for seed in [13, 37, 71] {
			let table = Puff::table(seed);
			for p in table {
				assert!(p.state(p.birth - 1e-6).is_none());
				assert!(p.state(p.birth).is_some());
				assert!(p.state(p.birth + p.lifetime + 1e-6).is_none());
				let mut t = 0.;
				for _ in 0..600 {
					t += 0.001;
				}
				let a = p.state(0.6);
				let b = p.state(t);
				assert_eq!(a.is_some(), b.is_some());
				if let (Some(a), Some(b)) = (a, b) {
					assert!((a.transform.translation - b.transform.translation).length() < 1e-6);
				}
			}
		}
	}
}
