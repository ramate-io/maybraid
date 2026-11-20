//! Analytic port of delivered wave_motion.py/local_deformation.py (metres, +X/+Y/+Z).
//! NumPy `q @ R.T` becomes column-vector `R * q`; yaw therefore uses -yaw in Bevy.
use bevy::prelude::*;
use std::sync::Arc;

const KEYS: [[[f32; 5]; 5]; 3] = [
	[
		[0., 0., 0., 0., 0.],
		[0.35, 0.28, -0.12, 0.04, -0.03],
		[1.1, 0.18, 0.42, 0.18, 0.10],
		[2., -0.12, 0.31, -0.08, -0.06],
		[3., 0.04, 0.22, 0.06, 0.02],
	],
	[
		[0., 0., 0., 0., 0.],
		[0.5, -0.10, -0.08, 0.12, 0.06],
		[1.45, 0.30, 0.38, -0.05, -0.10],
		[2.3, 0.12, 0.45, 0.16, 0.04],
		[3., 0.03, 0.27, 0.06, 0.01],
	],
	[
		[0., 0., 0., 0., 0.],
		[0.25, 0.18, -0.08, -0.08, 0.04],
		[0.85, 0.32, 0.32, 0.16, -0.08],
		[1.8, -0.08, 0.46, 0.08, 0.10],
		[3., 0.02, 0.24, 0.04, 0.01],
	],
];
pub fn smooth(x: f32) -> f32 {
	let u = x.clamp(0., 1.);
	u * u * (3. - 2. * u)
}
#[derive(Clone, Copy, Debug)]
pub struct Deformation(pub Vec4);
impl Deformation {
	pub fn at(phase: f32, clip: usize) -> Self {
		let t = phase.clamp(0., 3.);
		let keys = &KEYS[clip];
		let i = (0..4).find(|&i| t < keys[i + 1][0]).unwrap_or(3);
		let a = keys[i];
		let b = keys[i + 1];
		let u = smooth((t - a[0]) / (b[0] - a[0]));
		Self(Vec4::new(a[1], a[2], a[3], a[4]).lerp(Vec4::new(b[1], b[2], b[3], b[4]), u))
	}
	pub fn map(self, p: Vec3) -> Vec3 {
		let Vec3 { x, y, z } = p;
		let c = self.0;
		let f = (-((x - 0.40) / 0.65).powi(2) - (z / 0.85).powi(2)).exp()
			- 0.6 * (-((x + 0.55) / 0.60).powi(2) - (z / 0.85).powi(2)).exp();
		let yy = y * (c.x * f).exp();
		let h = yy * (-0.5 * (yy / 1.25).powi(2)).exp();
		let q = (-((yy - 0.75) / 0.65).powi(2)).exp();
		Vec3::new(x + c.y * h, yy, z * (c.z * q).exp() + c.w * h)
	}
	pub fn inverse(self, p: Vec3) -> Vec3 {
		let c = self.0;
		let h = p.y * (-0.5 * (p.y / 1.25).powi(2)).exp();
		let q = (-((p.y - 0.75) / 0.65).powi(2)).exp();
		let x = p.x - c.y * h;
		let z = (p.z - c.w * h) / ((c.z * q).exp());
		let f = (-((x - 0.40) / 0.65).powi(2) - (z / 0.85).powi(2)).exp()
			- 0.6 * (-((x + 0.55) / 0.60).powi(2) - (z / 0.85).powi(2)).exp();
		Vec3::new(x, p.y / (c.x * f).exp(), z)
	}
	pub fn normal_analytic(self, p: Vec3, n: Vec3) -> Vec3 {
		let c = self.0;
		let a = (p.x - 0.4) / 0.65;
		let b = (p.x + 0.55) / 0.60;
		let z = p.z / 0.85;
		let f1 = (-a * a - z * z).exp();
		let f2 = (-b * b - z * z).exp();
		let f = f1 - 0.6 * f2;
		let fx = -2. * a / 0.65 * f1 + 1.2 * b / 0.60 * f2;
		let fz = -2. * p.z / (0.85 * 0.85) * f;
		let ey = (c.x * f).exp();
		let y = p.y * ey;
		let dy = Vec3::new(y * c.x * fx, ey, y * c.x * fz);
		let hp = (-0.5 * (y / 1.25).powi(2)).exp() * (1. - (y / 1.25).powi(2));
		let q = (-((y - 0.75) / 0.65).powi(2)).exp();
		let qp = q * (-2. * (y - 0.75) / (0.65 * 0.65));
		let ez = (c.z * q).exp();
		let k = p.z * ez * c.z * qp + c.w * hp;
		let gx = Vec3::X + dy * (c.y * hp);
		let gz = Vec3::Z * ez + dy * k;
		let j = Mat3::from_cols(
			Vec3::new(gx.x, dy.x, gz.x),
			Vec3::new(gx.y, dy.y, gz.y),
			Vec3::new(gx.z, dy.z, gz.z),
		);
		(j.inverse().transpose() * n).normalize()
	}

	pub fn normal(self, p: Vec3, n: Vec3) -> Vec3 {
		// Same centred Jacobian probe as the reference; the shader uses these same offsets.
		let e = 0.0005;
		let j = Mat3::from_cols(
			(self.map(p + Vec3::X * e) - self.map(p - Vec3::X * e)) / (2. * e),
			(self.map(p + Vec3::Y * e) - self.map(p - Vec3::Y * e)) / (2. * e),
			(self.map(p + Vec3::Z * e) - self.map(p - Vec3::Z * e)) / (2. * e),
		);
		(j.inverse().transpose() * n).normalize()
	}
}
#[derive(Clone, Copy, Debug)]
pub struct Pose {
	pub position: Vec3,
	pub scale: Vec3,
	pub rotation: Quat,
	pub pivot: Vec3,
	pub coefficients: Deformation,
	pub variant: usize,
	pub phase: f32,
	pub clip: usize,
	pub birth: f32,
	pub yaw: f32,
	pub support: f32,
	pub tilt: f32,
}
impl Pose {
	pub fn world(self, p: Vec3) -> Vec3 {
		self.rotation * (self.coefficients.map(p) * self.scale - self.pivot)
			+ self.pivot
			+ self.position
	}
	pub fn normal(self, p: Vec3, n: Vec3) -> Vec3 {
		(self.rotation * (self.coefficients.normal(p, n) / self.scale)).normalize()
	}
	pub fn rest(self, world: Vec3) -> Vec3 {
		self.coefficients.inverse(
			(self.rotation.conjugate() * (world - self.position - self.pivot) + self.pivot)
				/ self.scale,
		)
	}
	/// Conservative local deformation envelope, then exact pivot/TRS corner mapping.
	pub fn bounds(self, lo: Vec3, hi: Vec3) -> (Vec3, Vec3) {
		let c = self.coefficients.0.abs();
		let h = 1.25 * (-0.5f32).exp();
		let min = Vec3::new(
			lo.x - c.y * h,
			(lo.y * c.x.exp()).min(lo.y * (-c.x).exp()),
			lo.z * c.z.exp() - c.w * h,
		);
		let max = Vec3::new(
			hi.x + c.y * h,
			(hi.y * c.x.exp()).max(hi.y * (-c.x).exp()),
			hi.z * c.z.exp() + c.w * h,
		);
		let transform = self.transform();
		let mut a = Vec3::splat(f32::INFINITY);
		let mut b = -a;
		for i in 0..8 {
			let p = Vec3::new(
				if i & 1 == 0 { min.x } else { max.x },
				if i & 2 == 0 { min.y } else { max.y },
				if i & 4 == 0 { min.z } else { max.z },
			);
			let w = transform.transform_point(p);
			a = a.min(w);
			b = b.max(w);
		}
		(a, b)
	}
	pub fn transform(self) -> Transform {
		Transform {
			translation: self.position + self.pivot - self.rotation * self.pivot,
			rotation: self.rotation,
			scale: self.scale,
		}
	}
	fn base(t: f32, i: usize) -> Option<Self> {
		let group = (i / 4) as f32;
		let lane = (i % 4) as f32;
		let birth = group * 0.40 + lane * 0.055;
		let age = t - birth;
		if age < 0. {
			return None;
		}
		let yaw = (lane - 1.5) * 0.28;
		let distance =
			2.25 * (1. - (-age / (0.52 + group * 0.06)).exp()) + group * 0.14 * smooth(age / 1.2);
		let lateral =
			(lane - 1.5) * 0.46 + 0.13 * (age * 1.3 + group * 0.65).sin() * smooth(age / 0.4);
		let wobble = 1.9 * age + group * 0.85;
		let position = Vec3::new(
			0.22 + group * 0.12 + distance - 1.4 + 0.24 * wobble.sin() * smooth(age / 0.35),
			0.025
				+ 0.24 * (std::f32::consts::PI * (age / 2.3).clamp(0., 1.)).sin().powi(2)
				+ 0.14 * (0.5 + 0.5 * wobble.sin()) * smooth(age / 0.5),
			lateral,
		);
		let scale = Vec3::new(0.64, 0.54, 0.63)
			* (0.78 + 0.12 * (i as f32 * 0.73).cos())
			* (0.12 + 0.88 * smooth(age / 0.36));
		let phase = (age * (0.73 + 0.08 * lane) + group * 0.16).clamp(0., 3.);
		let clip = (i % 4 + i / 4) % 3;
		Some(Self {
			position,
			scale,
			rotation: Quat::from_rotation_y(-yaw),
			pivot: Vec3::ZERO,
			coefficients: Deformation::at(phase, clip),
			variant: i % 3,
			phase,
			clip,
			birth,
			yaw,
			support: 0.,
			tilt: 0.,
		})
	}
}
#[derive(Clone, Copy)]
struct ChildRule {
	parent: usize,
	birth: f32,
	anchor: Vec3,
	ratio: f32,
	extra: Vec3,
	clip: usize,
	offset: f32,
	speed: f32,
}
const CHILDREN: [ChildRule; 3] = [
	ChildRule {
		parent: 3,
		birth: 1.22,
		anchor: Vec3::new(0.25, 0.64, 0.18),
		ratio: 0.34,
		extra: Vec3::new(0.16, 0.07, 0.12),
		clip: 1,
		offset: 0.13,
		speed: 0.88,
	},
	ChildRule {
		parent: 0,
		birth: 2.08,
		anchor: Vec3::new(0.18, 0.76, -0.25),
		ratio: 0.30,
		extra: Vec3::new(0.21, 0.17, -0.24),
		clip: 2,
		offset: 0.29,
		speed: 0.94,
	},
	ChildRule {
		parent: 9,
		birth: 2.71,
		anchor: Vec3::new(0.28, 0.78, -0.20),
		ratio: 0.32,
		extra: Vec3::new(0.22, 0.16, -0.20),
		clip: 0,
		offset: 0.07,
		speed: 0.81,
	},
];
#[derive(Clone, Copy, Debug)]
pub struct ChildInitial {
	pub anchor: Vec3,
	pub velocity: Vec3,
	pub parent: Pose,
}
/// Immutable rest positions are shared across instances; birth material values freeze once.
pub struct Wave {
	pub rest: [Arc<[Vec3]>; 3],
	pub children: [ChildInitial; 3],
}
impl Wave {
	pub fn new(rest: [Arc<[Vec3]>; 3]) -> Self {
		let placeholder = ChildInitial {
			anchor: Vec3::ZERO,
			velocity: Vec3::ZERO,
			parent: Pose {
				position: Vec3::ZERO,
				scale: Vec3::ONE,
				rotation: Quat::IDENTITY,
				pivot: Vec3::ZERO,
				coefficients: Deformation(Vec4::ZERO),
				variant: 0,
				phase: 0.,
				clip: 0,
				birth: 0.,
				yaw: 0.,
				support: 0.,
				tilt: 0.,
			},
		};
		let mut wave = Self { rest, children: [placeholder; 3] };
		for (k, r) in CHILDREN.iter().enumerate() {
			// All selected parents are born long before these fixed child births.
			if let (Some(parent), Some(before), Some(after)) = (
				wave.parent(r.birth, r.parent),
				wave.parent(r.birth - 0.001, r.parent),
				wave.parent(r.birth + 0.001, r.parent),
			) {
				wave.children[k] = ChildInitial {
					anchor: parent.world(r.anchor),
					velocity: (after.world(r.anchor) - before.world(r.anchor)) / 0.002,
					parent,
				};
			}
		}
		wave
	}
	pub fn parent(&self, t: f32, i: usize) -> Option<Pose> {
		let mut s = Pose::base(t, i)?;
		let rule: Option<(f32, f32, f32, f32, f32)> = match i {
			0 => Some((1.50, 1.65, 7., 0.13, -0.065)),
			3 => Some((1.72, 1.55, 8., 0.15, 0.075)),
			5 => Some((1.88, 1.45, 6., 0.11, -0.045)),
			9 => Some((1.58, 1.80, 9., 0.16, -0.075)),
			_ => None,
		};
		if let Some((start, duration, degrees, dx, dz)) = rule {
			let u = smooth((t - s.birth - start) / duration);
			if u > 0. {
				let points = &self.rest[s.variant];
				let floor = points.iter().map(|&p| s.world(p).y).fold(f32::INFINITY, f32::min);
				s.pivot = Vec3::new(0.10, 0.12, 0.) * s.scale;
				let yaw = Quat::from_rotation_y(-s.yaw);
				s.tilt = degrees * u;
				s.rotation = yaw
					* Quat::from_axis_angle(
						Vec3::new(dz.signum() * 0.24, 0., -1.).normalize(),
						s.tilt.to_radians(),
					);
				s.position += yaw * Vec3::new(dx * u, 0.035 * u * u, dz * u * u);
				s.support =
					floor - points.iter().map(|&p| s.world(p).y).fold(f32::INFINITY, f32::min);
				s.position.y += s.support;
			}
		}
		Some(s)
	}
	pub fn child(&self, t: f32, k: usize) -> Option<Pose> {
		let r = CHILDREN[k];
		let age = t - r.birth;
		if age < 0. {
			return None;
		}
		let init = self.children[k];
		let size = 0.04 + (r.ratio - 0.04) * smooth(age / 0.68);
		let scale = init.parent.scale * size;
		let phase = (r.offset + age * r.speed).clamp(0., 3.);
		let coefficients = Deformation::at(phase, r.clip);
		let rotation = Quat::from_rotation_y(-init.parent.yaw);
		let center = coefficients.map(Vec3::new(0.05, 0.66, 0.)) * scale;
		let motion = 0.65 * init.velocity * 0.85 * (1. - (-age / 0.85).exp());
		let extra = rotation * r.extra * smooth(age / 1.12);
		let current_anchor = self.parent(t, r.parent)?.world(r.anchor);
		let attachment = 0.85 * smooth(age / 0.72);
		let shoulder = init.anchor + motion + attachment * (current_anchor - init.anchor - motion);
		Some(Pose {
			position: shoulder + extra - rotation * center,
			scale,
			rotation,
			pivot: Vec3::ZERO,
			coefficients,
			variant: init.parent.variant,
			phase,
			clip: r.clip,
			birth: r.birth,
			yaw: init.parent.yaw,
			support: 0.,
			tilt: 0.,
		})
	}
	/// Shape channels for already-released interactive bodies; no authored support scans.
	pub fn released_shape(&self, t: f32, index: usize) -> Option<Pose> {
		if index < 12 {
			return Pose::base(t, index);
		}
		let k = index - 12;
		let r = *CHILDREN.get(k)?;
		let age = t - r.birth;
		if age < 0. {
			return None;
		}
		let init = self.children[k];
		let phase = (r.offset + age * r.speed).clamp(0., 3.);
		Some(Pose {
			position: Vec3::ZERO,
			scale: init.parent.scale * (0.04 + (r.ratio - 0.04) * smooth(age / 0.68)),
			rotation: Quat::from_rotation_y(-init.parent.yaw),
			pivot: Vec3::ZERO,
			coefficients: Deformation::at(phase, r.clip),
			phase,
			clip: r.clip,
			birth: r.birth,
			support: 0.,
			tilt: 0.,
			..init.parent
		})
	}

	pub fn states(&self, t: f32) -> [Option<Pose>; 15] {
		std::array::from_fn(|i| if i < 12 { self.parent(t, i) } else { self.child(t, i - 12) })
	}
}
#[cfg(test)]
mod tests;
