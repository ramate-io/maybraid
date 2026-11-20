//! Delivered dynamic_dust.py port. The host supplies target EFFECT time; no wall clock.
use crate::{
	composition::CombinedScenario,
	parts::{
		cartoon_wave::motion::{Pose, Wave},
		combined::assets::Anchor,
		packaged::PackagedAssetError,
	},
};
use bevy::{
	math::{DMat4, DQuat, DVec3},
	prelude::*,
};
use serde::Deserialize;
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
	pub dt: f64,
	pub seed: u32,
	pub cloud_rate_per_region_s: f64,
	pub grain_rate_per_region_s: f64,
	pub max_clouds: usize,
	pub max_grains: usize,
	pub guide_rate_per_s: f64,
	pub guide_age_decay_s: f64,
	pub guide_reach_m: f64,
	pub concentration: f64,
	pub wind_scale: f64,
}
impl Config {
	pub fn validate(self) -> Result<(), PackagedAssetError> {
		if !self.dt.is_finite()
			|| (self.dt - 1. / 120.).abs() > 1e-12
			|| self.max_clouds > 96
			|| self.max_grains > 96
			|| self.max_clouds == 0
			|| self.max_grains == 0
			|| [
				self.cloud_rate_per_region_s,
				self.grain_rate_per_region_s,
				self.guide_rate_per_s,
				self.guide_age_decay_s,
				self.guide_reach_m,
				self.concentration,
				self.wind_scale,
			]
			.iter()
			.any(|v| !v.is_finite() || *v < 0.)
			|| self.guide_age_decay_s == 0.
			|| self.guide_reach_m == 0.
		{
			return Err(PackagedAssetError::invalid(
				"Combined requires120Hz and bounded valid dynamic configuration",
			));
		}
		Ok(())
	}
}
pub fn mix32(mut x: u32) -> u32 {
	x ^= x >> 16;
	x = x.wrapping_mul(0x7feb352d);
	x ^= x >> 15;
	x = x.wrapping_mul(0x846ca68b);
	x ^= x >> 16;
	x
}
pub fn random(seed: u32, region: u32, tick: u32, channel: u32) -> f64 {
	mix32(
		seed ^ mix32(region.wrapping_add(0x9e3779b9))
			^ mix32(tick.wrapping_add(0x85ebca6b))
			^ mix32(channel.wrapping_add(0x27d4eb2d)),
	) as f64
		/ 4294967296.
}
pub fn smooth(x: f64) -> f64 {
	let u = x.clamp(0., 1.);
	u * u * (3. - 2. * u)
}
#[derive(Clone, Debug)]
pub struct Particle {
	pub id: u32,
	pub parent: usize,
	pub region: usize,
	pub kind: u32,
	pub variant: usize,
	pub birth: f64,
	pub life: f64,
	pub position: DVec3,
	pub velocity: DVec3,
	pub radius: f64,
	pub rotation: DQuat,
	pub drag: f64,
	pub gravity: f64,
	pub base_concentration: f64,
	pub spin: f64,
	pub scale: DVec3,
	pub concentration: f64,
	pub age: f64,
	pub color: DVec3,
}
#[derive(Clone, Debug)]
pub struct Birth {
	pub id: u32,
	pub rest: DVec3,
	pub position: DVec3,
	pub inherited: DVec3,
	pub release: DVec3,
}
#[derive(Clone, Copy)]
struct Region {
	id: usize,
	parent: usize,
	center: Vec3,
	normal: Vec3,
	radii: Vec3,
	gate: f64,
	pose: Pose,
	before: Pose,
	after: Pose,
	lo: f64,
	hi: f64,
	placement: DMat4,
}
impl Region {
	fn velocity(self, rest: Vec3) -> DVec3 {
		self.placement.transform_vector3(
			(self.after.world(rest).as_dvec3() - self.before.world(rest).as_dvec3())
				/ (self.hi - self.lo),
		)
	}
	fn world(self) -> DVec3 {
		self.placement.transform_point3(self.pose.world(self.center).as_dvec3())
	}
}
const REGIONS: [(usize, usize, f64, f64, f64); 10] = [
	(0, 1, 0.18, 1.10, 1.),
	(0, 0, 1.65, 2.55, 0.65),
	(3, 0, 0.26, 1.35, 1.2),
	(3, 2, 0.38, 1.55, 0.80),
	(6, 1, 0.24, 1.50, 1.1),
	(6, 0, 0.50, 1.55, 0.60),
	(9, 0, 0.26, 1.50, 1.),
	(9, 2, 0.42, 1.65, 0.70),
	(11, 1, 0.30, 1.60, 0.85),
	(13, 0, 0.16, 0.55, 0.6),
];
pub struct Simulation {
	pub interactive: Option<crate::parts::combined::interactive::Interactive>,
	pub placement: DMat4,
	pub body: Wave,
	pub config: Config,
	pub scenario: CombinedScenario,
	pub tick: u32,
	pub particles: Vec<Particle>,
	pub dropped: [u32; 2],
	pub anchors: [[Anchor; 3]; 3],
	// Birth diagnostic is bounded by the finite emitter's480 ticks*10 regions*2 kinds.
	#[cfg(test)]
	pub births: Vec<Birth>,
}
impl Simulation {
	pub fn new(
		body: Wave,
		config: Config,
		anchors: [[Anchor; 3]; 3],
		scenario: CombinedScenario,
	) -> Self {
		Self {
			placement: DMat4::IDENTITY,
			interactive: None,
			body,
			config,
			scenario,
			tick: 0,
			particles: Vec::with_capacity(config.max_clouds + config.max_grains),
			dropped: [0; 2],
			anchors,
			#[cfg(test)]
			births: Vec::new(),
		}
	}
	pub fn time(&self) -> f64 {
		self.tick as f64 * self.config.dt
	}
	pub fn pose(&self, t: f64, id: usize) -> Option<Pose> {
		if let Some(s) = &self.interactive {
			return s.pose(id);
		}
		let mut s = if id < 12 {
			self.body.parent(t as f32, id)
		} else {
			self.body.child(t as f32, id - 12)
		}?;
		if self.scenario == CombinedScenario::WindTurn {
			let g = Quat::from_rotation_y((-0.65 * smooth((t - 1.15) / 1.3)) as f32);
			s.rotation = g * s.rotation;
			s.position = g * (s.position + s.pivot) - s.pivot;
		}
		Some(s)
	}
	fn regions(&self, t: f64) -> Vec<Region> {
		if t > 2.82 {
			return Vec::new();
		}
		let mut out = Vec::with_capacity(10);
		for (id, &(parent, kind, start, end, amplitude)) in REGIONS.iter().enumerate() {
			let Some(pose) = self.pose(t, parent) else {
				continue;
			};
			let age = t - pose.birth as f64;
			if !(start < age && age < end) {
				continue;
			}
			let gate =
				(std::f64::consts::PI * (age - start) / (end - start)).sin().powf(1.25) * amplitude;
			let a = self.anchors[pose.variant][kind];
			let normal = Vec3::from_array(a.normal);
			let center = Vec3::from_array(a.point) + normal * 0.16;
			let radii = [
				Vec3::new(0.34, 0.25, 0.34),
				Vec3::new(0.40, 0.21, 0.31),
				Vec3::new(0.39, 0.18, 0.37),
			][kind];
			let lo = (t - 0.0005).max(pose.birth as f64);
			let hi = t + 0.0005;
			if let (Some(before), Some(after)) = (self.pose(lo, parent), self.pose(hi, parent)) {
				out.push(Region {
					id,
					parent,
					center,
					normal,
					radii,
					gate,
					pose,
					before,
					after,
					lo,
					hi,
					placement: self.placement,
				});
			}
		}
		out
	}
	pub fn ambient(p: DVec3, t: f64, scenario: CombinedScenario, wind_scale: f64) -> DVec3 {
		let mut wind = DVec3::new(0.18, 0., 0.02);
		if scenario == CombinedScenario::WindTurn {
			wind = wind.lerp(DVec3::new(-0.15, 0.03, 0.75), smooth((t - 1.4) / 0.65));
		}
		let mut flow = wind * wind_scale;
		for (center, axis, radius, strength) in [
			(DVec3::new(0.45, 0.45, 0.), DVec3::new(0., 0., -1.), 1.10, 0.70),
			(DVec3::new(1.35, 0.65, -0.35), DVec3::new(0.15, 1., 0.1), 0.85, 0.42),
		] {
			let d = p - center - DVec3::new(0.08 * (t * 0.8).sin(), 0., 0.06 * (t * 0.6).sin());
			let w = (1. - d.length_squared() / (radius * radius)).max(0.).powi(2);
			flow += axis.normalize().cross(d) * strength * w;
		}
		flow
	}
	fn spawn(&self, r: Region, kind: u32, t: f64) -> (Particle, Birth) {
		let c = self.config;
		let rand = |ch| random(c.seed, r.id as u32, self.tick, ch + kind * 40);
		let id = kind * (1 << 30) + r.id as u32 * 100000 + self.tick;
		let direction =
			DVec3::new(2. * rand(1) - 1., 2. * rand(2) - 1., 2. * rand(3) - 1.).normalize_or_zero();
		let rest =
			r.center.as_dvec3() + direction * rand(4).powf(1. / 3.) * r.radii.as_dvec3() * 0.7;
		let position = self.placement.transform_point3(r.pose.world(rest.as_vec3()).as_dvec3());
		let inherited = r.velocity(rest.as_vec3());
		let n = r.pose.normal(rest.as_vec3(), r.normal).as_dvec3();
		let release = self.placement.transform_vector3(
			0.19 * n + (r.pose.rotation * Vec3::new(-0.055, 0.075, 0.)).as_dvec3(),
		);
		let radius = self.placement.x_axis.truncate().length()
			* if kind == 0 { 0.10 + 0.09 * rand(5) } else { 0.0021 + 0.0023 * rand(5) };
		let life = if kind == 0 { 1. + 0.8 * rand(6) } else { 0.65 + 0.8 * rand(6) };
		let rotation = self.placement.to_scale_rotation_translation().1
			* DQuat::from_rotation_y(2. * std::f64::consts::PI * rand(7))
			* DQuat::from_rotation_x(0.7 * (rand(8) - 0.5));
		(
			Particle {
				id,
				parent: r.parent,
				region: r.id,
				kind,
				variant: if kind == 0 { (rand(9) * 4.) as usize } else { 0 },
				birth: t,
				life,
				position,
				velocity: inherited + release,
				radius,
				rotation,
				drag: if kind == 0 { 1.5 + 1.3 * rand(10) } else { 2. + 3. * rand(10) },
				gravity: if kind == 0 { 0. } else { 0.16 + 0.20 * rand(11) },
				base_concentration: c.concentration * (0.8 + 0.4 * rand(12)),
				spin: 0.18 + 0.35 * rand(13),
				scale: DVec3::splat(radius),
				concentration: 0.,
				age: 0.,
				color: DVec3::new(0.65, 0.44, 0.25)
					* if kind == 0 { 1. } else { 0.36 + 0.28 * rand(14) },
			},
			Birth { id, rest, position, inherited, release },
		)
	}
	pub fn step(&mut self) {
		let c = self.config;
		let t = self.time();
		let dt = c.dt;
		self.particles.retain(|p| t < p.birth + p.life);
		let regions = self.regions(t);
		let mut counts = [
			self.particles.iter().filter(|p| p.kind == 0).count(),
			self.particles.iter().filter(|p| p.kind == 1).count(),
		];
		for &r in &regions {
			for (kind, rate, capacity) in [
				(0, c.cloud_rate_per_region_s, c.max_clouds),
				(1, c.grain_rate_per_region_s, c.max_grains),
			] {
				if random(c.seed, r.id as u32, self.tick, kind * 40) >= rate * dt * r.gate {
					continue;
				}
				if counts[kind as usize] >= capacity {
					self.dropped[kind as usize] += 1;
					continue;
				}
				let (particle, _birth) = self.spawn(r, kind, t);
				self.particles.push(particle);
				counts[kind as usize] += 1;
				#[cfg(test)]
				self.births.push(_birth);
			}
		}
		let guides = regions
			.iter()
			.map(|r| (r.world(), r.velocity(r.center), r.gate))
			.collect::<Vec<_>>();
		for p in &mut self.particles {
			let flow = Self::ambient(p.position, t + dt / 2., self.scenario, c.wind_scale);
			let mut weights = 0.;
			let mut moment = DVec3::ZERO;
			for &(center, velocity, gate) in &guides {
				let w = gate
					* (1. - (p.position - center).length_squared() / c.guide_reach_m.powi(2))
						.max(0.)
						.powi(2);
				weights += w;
				moment += w * velocity;
			}
			let target_body = moment / weights.max(1e-12);
			let following = (1. - (-weights).exp()) * (-(t - p.birth) / c.guide_age_decay_s).exp();
			let kbody = c.guide_rate_per_s * following;
			let k = p.drag + kbody;
			let target = (p.drag * flow + kbody * target_body + DVec3::new(0., -p.gravity, 0.)) / k;
			let velocity = target + (p.velocity - target) * (-k * dt).exp();
			p.position += 0.5 * (p.velocity + velocity) * dt;
			p.velocity = velocity;
			p.age = t + dt - p.birth;
			let a = p.age / p.life;
			let floor = if p.kind == 0 {
				let grow = (0.3 + 0.7 * smooth(p.age / 0.18)) * (1. + 0.4 * a);
				p.scale = p.radius * grow * DVec3::new(1.12, 0.80, 1.);
				p.concentration = p.base_concentration
					* smooth(p.age / 0.12)
					* (-0.55 * p.age).exp()
					* (1. - smooth((a - 0.65) / 0.35));
				p.rotation =
					(DQuat::from_axis_angle(-DVec3::Z, p.spin * dt) * p.rotation).normalize();
				p.scale.y
			} else {
				p.scale = p.radius * DVec3::new(1., 0.66, 1.23);
				p.concentration = 1.;
				p.radius
			};
			if p.position.y < floor {
				p.position.y = floor;
				p.velocity.y = (-p.velocity.y * 0.12).max(0.);
				p.velocity.x *= 0.8;
				p.velocity.z *= 0.8;
			}
		}
		self.tick += 1;
	}
	/// Catch up at most32 ticks/frame. All remaining target time is retained by the host.
	/// Return backlog in seconds; host retirement waits for the simulation to reach4.7s.
	pub fn advance_interactive(
		&mut self,
		target: f64,
		scene: &crate::parts::combined::interaction::InteractionScene,
	) -> f64 {
		let goal = (target.max(0.) * 120. + 1e-6).floor() as u32;
		let end = goal.min(self.tick.saturating_add(32));
		if let Some(mut state) = self.interactive.take() {
			while self.tick < end {
				state.step(self, scene);
			}
			self.interactive = Some(state);
		}
		goal.saturating_sub(self.tick) as f64 * self.config.dt
	}
	pub fn advance_to(&mut self, target: f64) -> f64 {
		let goal = (target.max(0.) * 120. + 1e-6).floor() as u32;
		let end = goal.min(self.tick.saturating_add(32));
		while self.tick < end {
			self.step();
		}
		goal.saturating_sub(self.tick) as f64 * self.config.dt
	}
}
