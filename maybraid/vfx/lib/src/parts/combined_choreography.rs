//! Combined appearance for the approved Landing and refined contact controller.
//! No opaque surfaces are spawned. Particles sample complete consecutive material maps.
use crate::{
	composition::{CartoonFlowPart, CombinedChoreographyPart, CombinedLayers},
	parts::{
		body_flow::BodyMotion,
		cartoon_flow::CartoonFlow,
		combined::{
			assets::CombinedAsset,
			interaction::{InteractionScene, DT},
			render::Primitive,
			simulation::{random, smooth, Particle, Simulation},
		},
		packaged::StaticVolume,
	},
	spawn::VfxSpawn,
};
use bevy::{
	math::{DQuat, DVec3},
	prelude::*,
};
pub const MAX_CLOUDS: usize = 32;
pub const MAX_GRAINS: usize = 24;
#[derive(Component)]
pub struct OpticalChoreography {
	pub part: CombinedChoreographyPart,
	pub particles: Vec<Particle>,
	pub seed: u32,
	pub dropped: [u32; 2],
	pub ready: bool,
	pub body_ns: u64,
	pub particle_ns: u64,
	pub steps: u32,
}
impl CombinedChoreographyPart {
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) {
		let controller = CartoonFlowPart {
			meshes: self.meshes.clone(),
			landing: self.landing,
			refined: self.contact,
		};
		let entity = controller.spawn_controller(commands, parent, transform, spawn, self.contact);
		commands.entity(entity).insert(OpticalChoreography {
			part: self.clone(),
			particles: Vec::with_capacity(MAX_CLOUDS + MAX_GRAINS),
			seed: spawn.resolved_seed() as u32,
			dropped: [0; 2],
			ready: false,
			body_ns: 0,
			particle_ns: 0,
			steps: 0,
		});
	}
}
impl OpticalChoreography {
	pub fn duration(&self) -> f32 {
		self.part.duration()
	}
	pub fn contact(&self) -> bool {
		self.part.contact
	}
	pub fn enabled(&self, role: CombinedLayers) -> bool {
		self.part.options.layers == CombinedLayers::All || self.part.options.layers == role
	}
	pub(crate) fn dependencies(
		&self,
		assets: &Assets<CombinedAsset>,
		volumes: &Assets<StaticVolume>,
		images: &Assets<Image>,
	) -> bool {
		assets.get(&self.part.assets).is_some_and(|a| {
			a.bodies
				.iter()
				.chain(&a.clouds)
				.all(|h| volumes.get(h).is_some_and(|v| images.contains(&v.image)))
		})
	}
	pub(crate) fn valid_placement(&self, m: Mat4) -> bool {
		let o = self.part.options;
		if [o.body_concentration, o.fine_concentration]
			.iter()
			.any(|v| !v.is_finite() || !(0. ..=3.).contains(v))
			|| !o.cloud_rate.is_finite()
			|| !(0. ..=30.).contains(&o.cloud_rate)
			|| !o.grain_rate.is_finite()
			|| !(0. ..=15.).contains(&o.grain_rate)
		{
			return false;
		}
		if !m.is_finite() {
			return false;
		}
		if self.contact() {
			return crate::parts::cartoon_flow::region::ContactFrame::new(m).is_ok();
		}
		let (s, r, t) = m.to_scale_rotation_translation();
		s.min_element() > 0.
			&& s.is_finite()
			&& (r * Vec3::Y - Vec3::Y).length() < 1e-4
			&& t.y.abs() < 1e-5
			&& (s.x - s.y).abs() < 1e-4
			&& (s.x - s.z).abs() < 1e-4
	}
	/// Complete forward map and its finite-tick material velocity, including affine/warp.
	pub fn material(b: &BodyMotion, rest: Vec3) -> (DVec3, DVec3) {
		let now = b.model().transform_point3(b.pose.coefficients.map(rest));
		let before = b.previous_model.transform_point3(b.previous.coefficients.map(rest));
		(now.as_dvec3(), (now - before).as_dvec3() / DT)
	}
	pub(crate) fn step(
		&mut self,
		controller: &CartoonFlow,
		scene: &InteractionScene,
		field: Option<&crate::parts::combined::flow::Flow>,
		asset: &CombinedAsset,
	) {
		let bodies = &controller.bodies;
		let tick = controller.tick.saturating_sub(1);
		let t = tick as f64 * DT;
		self.particles.retain(|p| t < p.birth + p.life);
		let mut counts = [
			self.particles.iter().filter(|p| p.kind == 0).count(),
			self.particles.iter().filter(|p| p.kind == 1).count(),
		];
		// One anchor per parent per tick. Three refined sub-lobes share the same event budget.
		for (i, b) in bodies.iter().enumerate().filter_map(|(i, b)| b.as_ref().map(|b| (i, b))) {
			if self.contact() && i % 3 != 0 {
				continue;
			}
			let age = t - b.birth;
			let active = if self.part.landing {
				(0.045..0.52).contains(&age)
			} else {
				(0.12..2.6).contains(&age)
			};
			if !active {
				continue;
			}
			let a = asset.manifest.anchors[b.pose.variant][i % 3];
			let rest = Vec3::from_array(a.point);
			if self.part.landing {
				let (threshold, seed) = controller.retirement(i);
				if threshold > 0.
					&& crate::parts::cartoon_flow::surface::ClosedField::noise(rest, seed)
						<= threshold
				{
					continue;
				}
			}
			let (position, inherited) = Self::material(b, rest);
			let normal = (Mat3::from_mat4(b.model()).inverse().transpose()
				* b.pose.coefficients.normal_analytic(rest, Vec3::from_array(a.normal)))
			.normalize_or_zero()
			.as_dvec3();
			let factor = b.scale.max_element().clamp(0.05, 2.);
			for kind in 0..2u32 {
				let rand = |c| random(self.seed, i as u32, tick, c + kind * 40);
				let rate = if kind == 0 {
					self.part.options.cloud_rate
				} else {
					self.part.options.grain_rate
				};
				if rand(0) >= rate * DT {
					continue;
				}
				let cap = if kind == 0 { MAX_CLOUDS } else { MAX_GRAINS };
				if counts[kind as usize] >= cap {
					self.dropped[kind as usize] += 1;
					continue;
				}
				let radius = if kind == 0 {
					factor * (0.18 + 0.10 * rand(5))
				} else {
					factor * (0.008 + 0.006 * rand(5))
				};
				self.particles.push(Particle {
					id: kind * (1 << 30) + i as u32 * 100000 + tick,
					parent: i,
					region: i,
					kind,
					variant: (rand(9) * 4.) as usize,
					birth: t,
					life: if kind == 0 { 0.65 + 0.55 * rand(6) } else { 0.5 + 0.45 * rand(6) },
					position: position + normal * radius * 0.3,
					velocity: inherited + normal * 0.14 + DVec3::Y * 0.04,
					radius,
					rotation: DQuat::from_rotation_y(rand(7) * std::f64::consts::TAU),
					drag: if kind == 0 { 2.2 } else { 3.5 },
					gravity: if kind == 0 { 0. } else { 0.3 },
					base_concentration: 0.4 * (0.8 + 0.4 * rand(12)),
					spin: 0.2,
					scale: DVec3::splat(radius),
					concentration: 0.,
					age: 0.,
					color: DVec3::new(0.3, 0.17, 0.07),
				});
				counts[kind as usize] += 1;
			}
		}
		let contact = self.part.contact;
		for p in &mut self.particles {
			let flow = field.map_or_else(
				|| {
					Simulation::ambient(
						p.position,
						t,
						crate::composition::CombinedScenario::Normal,
						1.,
					)
				},
				|f| f.sample(p.position),
			);
			let guide = bodies.get(p.parent).and_then(|b| b.as_ref()).map(|b| {
				let a = asset.manifest.anchors[b.pose.variant][p.parent % 3];
				let (pos, v) = Self::material(b, Vec3::from_array(a.point));
				let w = (1. - p.position.distance(pos) / 0.48).clamp(0., 1.).powi(2)
					* (-p.age / 0.35).exp();
				(v, w)
			});
			let (g, w) = guide.unwrap_or((DVec3::ZERO, 0.));
			let k = p.drag + 4. * w;
			let target = (p.drag * flow + 4. * w * g - DVec3::Y * p.gravity) / k;
			let v = target + (p.velocity - target) * (-k * DT).exp();
			p.position += (p.velocity + v) * 0.5 * DT;
			p.velocity = v;
			p.age = t + DT - p.birth;
			let a = (p.age / p.life).clamp(0., 1.);
			if p.kind == 0 {
				p.scale = p.radius
					* (0.3 + 0.7 * smooth(p.age / 0.12))
					* (1. + 0.4 * a)
					* DVec3::new(1.12, 0.8, 1.);
				p.concentration =
					p.base_concentration * smooth(p.age / 0.08) * (1. - smooth((a - 0.6) / 0.4));
			} else {
				p.scale = DVec3::splat(p.radius);
				p.concentration = 1.;
			}
			let floor = if p.kind == 0 { p.scale.y } else { p.radius };
			if p.position.y < floor {
				p.position.y = floor;
				p.velocity.y = p.velocity.y.max(0.);
			}
			if contact {
				for c in scene.colliders(t).into_iter().take(scene.collider_count()) {
					let q = c.query(p.position);
					if q.distance < floor {
						p.position += q.normal * (floor - q.distance);
						p.velocity = c.remove_inward(p.position, p.velocity);
					}
				}
			}
		}
	}
	pub fn primitives(
		&self,
		controller: &CartoonFlow,
		supports: [(Vec3, Vec3); 3],
	) -> Vec<Primitive> {
		let mut out = Vec::with_capacity(48 + MAX_CLOUDS + MAX_GRAINS);
		if self.enabled(CombinedLayers::Body) {
			for (id, b) in controller
				.bodies
				.iter()
				.enumerate()
				.filter_map(|(i, b)| b.as_ref().map(|b| (i, b)))
			{
				let (lo, hi) =
					b.pose.bounds(supports[b.pose.variant].0, supports[b.pose.variant].1);
				let extra = b.model() * b.pose.transform().to_matrix().inverse();
				let (min, max) = Primitive::transformed_bounds(extra, lo, hi);
				let (threshold, landing_seed) = controller.retirement(id);
				let seed = if self.part.landing {
					landing_seed
				} else {
					crate::parts::combined::simulation::mix32(
						13 ^ (((id / 3) as u32 + 1).wrapping_mul(0x27d4eb2d)),
					)
				};
				out.push(Primitive {
					inverse: b.model().inverse(),
					masked: self.contact(),
					coefficients: if self.part.landing {
						Vec4::new(threshold, (seed & 65535) as f32, (seed >> 16) as f32, 0.)
					} else {
						b.pose.coefficients.0
					},
					parameters: Vec4::new(
						b.pose.variant as f32,
						(seed & 65535) as f32 * 0.000219,
						b.concentration as f32 * self.part.options.body_concentration,
						if self.part.landing { -1. } else { 0. },
					),
					min,
					max,
				});
			}
		}
		for p in &self.particles {
			if p.age >= p.life
				|| !self.enabled(if p.kind == 0 {
					CombinedLayers::Clouds
				} else {
					CombinedLayers::Grains
				}) {
				continue;
			}
			let model = Mat4::from_scale_rotation_translation(
				p.scale.as_vec3(),
				p.rotation.as_quat(),
				p.position.as_vec3(),
			);
			let (min, max) = Primitive::transformed_bounds(model, -Vec3::ONE, Vec3::ONE);
			out.push(Primitive {
				inverse: model.inverse(),
				masked: self.contact(),
				coefficients: Vec4::ZERO,
				parameters: if p.kind == 0 {
					Vec4::new(
						(p.variant + 3) as f32,
						0.,
						p.concentration as f32 * self.part.options.fine_concentration,
						1.,
					)
				} else {
					p.color.as_vec3().extend(2.)
				},
				min,
				max,
			});
		}
		out
	}
}
