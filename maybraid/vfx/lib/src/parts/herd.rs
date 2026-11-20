//! One bounded world-space dust region, fed by external timestamped inputs.
//! No actor paths, footstep animation, or per-puff velocity solvers.

pub mod emission;
pub mod group;
pub mod material;
pub mod terrain;
use crate::parts::cartoon_flow::diagnostics::{timed, Probe};
use crate::parts::herd::material::HerdMaterial;
use crate::{
	composition::HerdPart,
	membership::VfxMemberOf,
	parts::{
		body_flow::BodyMotion,
		cartoon_flow::{surface::ClosedFields, SurfaceEnvironment},
		cartoon_wave::motion::{Deformation, Pose},
		combined::{
			flow::Flow,
			interaction::{InteractionScene, Interactor, Wake, DT, MAX_COLLIDERS},
			render::Primitive,
			simulation::mix32,
		},
	},
	spawn::{VfxInstance, VfxLayerLife, VfxSpawn},
};
use bevy::{
	camera::{primitives::Aabb, visibility::NoAutoAabb},
	math::DVec3,
	prelude::*,
};
use emission::{Birth, DistanceEmitter, EmitterInput};
use std::collections::VecDeque;
pub const MAX_PUFFS: usize = 32;
pub const MAX_FINE: usize = 64;
pub const MAX_GRAINS: usize = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style {
	#[default]
	Cartoon,
	Combined,
	None,
}
#[derive(Clone, Debug)]
pub struct Settings {
	pub group: bool,
	pub travelling: bool,
	pub settling_accel: f64,
	pub settling_speed: f64,
	pub body_density: f32,
	pub fine_density: f32,
	pub coverage: f64,
	pub spacing: f64,
	pub size: f64,
	pub lifetime: f64,
	pub strength: f64,
	pub max_puffs: usize,
	pub max_fine: usize,
	pub max_grains: usize,
	pub style: Style,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			group: false,
			travelling: false,
			settling_accel: 0.12,
			settling_speed: 0.12,
			body_density: 1.,
			fine_density: 1.,
			coverage: 1.,
			spacing: 0.58,
			size: 0.18,
			lifetime: 0.8,
			strength: 1.,
			max_puffs: MAX_PUFFS,
			max_fine: MAX_FINE,
			max_grains: MAX_GRAINS,
			style: Style::Cartoon,
		}
	}
}
impl Settings {
	pub fn validate(&self) -> Result<(), &'static str> {
		if !self.settling_accel.is_finite()
			|| !(0. ..=0.5).contains(&self.settling_accel)
			|| !self.settling_speed.is_finite()
			|| !(0. ..=0.4).contains(&self.settling_speed)
			|| !self.body_density.is_finite()
			|| !(0.0..=4.).contains(&self.body_density)
			|| !self.fine_density.is_finite()
			|| !(0.0..=4.).contains(&self.fine_density)
			|| !self.coverage.is_finite()
			|| !(0.5..=1.3).contains(&self.coverage)
			|| !self.spacing.is_finite()
			|| !(0.15..=2.).contains(&self.spacing)
			|| !self.size.is_finite()
			|| !(0.06..=0.35).contains(&self.size)
			|| !self.lifetime.is_finite()
			|| !(0.3..=2.).contains(&self.lifetime)
			|| !self.strength.is_finite()
			|| !(0.0..=2.).contains(&self.strength)
			|| !(1..=MAX_PUFFS).contains(&self.max_puffs)
			|| self.max_fine > MAX_FINE
			|| self.max_grains > MAX_GRAINS
		{
			return Err(
				"invalid herd settings: spacing 0.15–2m, size 0.06–0.35, life 0.3–2s, strength 0–2, budget 1–32",
			);
		}
		Ok(())
	}
}
#[derive(Clone, Debug)]
pub struct Frame {
	/// Optional body/shoulder heights above local ground, in world metres, keyed by actor ID.
	pub heights: Vec<(u32, f64)>,
	pub time: f64,
	pub emitters: Vec<EmitterInput>,
	pub colliders: Vec<Interactor>,
	pub births: Vec<Birth>,
	pub style: Style,
	pub emitting: bool,
}
impl Frame {
	/// `time` is effect age; convert external world/game-second rates exactly once.
	/// Positions, birth ages and dimensions are already in world metres/effect age.
	pub fn from_world_rates(mut self, playback: f64) -> Result<Self, &'static str> {
		if !playback.is_finite() || playback <= 0. {
			return Err("positive finite playback required");
		}
		for e in &mut self.emitters {
			e.velocity /= playback;
		}
		for c in &mut self.colliders {
			c.linear /= playback;
			c.angular /= playback;
		}
		for b in &mut self.births {
			b.velocity /= playback;
		}
		Ok(self)
	}
}
/// Queue exact simulation snapshots before advancing the host. Producers may use
/// any movement system. Reject oversized/invalid input rather than silently truncate.
#[derive(Component, Default)]
pub struct Inputs {
	pub frames: VecDeque<Frame>,
	last_time: f64,
}
impl Inputs {
	pub fn push(&mut self, frame: Frame) -> Result<(), &'static str> {
		if (frame.time - self.last_time - DT).abs() > 1e-6 {
			return Err("herd snapshots must be consecutive 120Hz samples");
		}
		if frame.heights.len() > 20
			|| frame.heights.iter().any(|(_, h)| !h.is_finite() || *h <= 0.)
			|| frame.colliders.len() > MAX_COLLIDERS
			|| frame.emitters.len() > 20
			|| frame.births.len() > 64
			|| self.frames.len() >= 512
		{
			return Err("herd input budget exceeded");
		}
		if !frame.time.is_finite()
			|| self.frames.back().is_some_and(|p| p.time >= frame.time)
			|| frame.emitters.iter().any(|e| {
				!e.position.is_finite() || !e.velocity.is_finite() || !e.ground.normal.is_finite()
			}) || frame.colliders.iter().any(|c| !c.position.is_finite() || !c.linear.is_finite())
		{
			return Err("invalid herd snapshot");
		}
		if frame
			.emitters
			.iter()
			.enumerate()
			.any(|(i, e)| frame.emitters[..i].iter().any(|p| p.id == e.id))
			|| frame.births.iter().any(|b| {
				!b.position.is_finite()
					|| !b.velocity.is_finite()
					|| !b.normal.is_finite()
					|| !b.time.is_finite()
					|| b.time > frame.time + 1e-8
					|| b.time < frame.time - DT - 1e-8
			}) || frame.colliders.iter().any(|c| {
			!c.rotation.is_finite()
				|| !c.angular.is_finite()
				|| !c.strength.is_finite()
				|| match c.shape {
					crate::parts::combined::interaction::Shape::Capsule {
						radius,
						half_segment,
					} => {
						!radius.is_finite()
							|| !half_segment.is_finite()
							|| radius <= 0. || half_segment < 0.
					}
					crate::parts::combined::interaction::Shape::Box { half } => {
						!half.is_finite() || half.min_element() <= 0.
					}
				}
		}) {
			return Err("invalid herd IDs, birth interval, collider or domain placement");
		}
		self.last_time = frame.time;
		self.frames.push_back(frame);
		Ok(())
	}
}
struct Puff {
	aspect: f64,
	body: BodyMotion,
	life: f64,
	size: f64,
	normal: DVec3,
	style: Style,
	side: f64,
	release: u8,
	seed: u32,
	tile: usize,
	ground: Option<terrain::GroundSample>,
	ground_cache: Option<std::sync::Arc<terrain::Heightfield>>,
}
struct Fine {
	id: u64,
	position: DVec3,
	velocity: DVec3,
	age: f64,
	life: f64,
	size: f64,
	grain: bool,
	seed: u32,
	tile: usize,
	ground: Option<terrain::GroundSample>,
	ground_cache: Option<std::sync::Arc<terrain::Heightfield>>,
}
struct Slot {
	entity: Entity,
	material: Handle<HerdMaterial>,
	mesh: Handle<Mesh>,
	noise: Vec<f32>,
	nodes: Vec<Vec3>,
	drawn: Option<Handle<Mesh>>,
	puff: Option<Puff>,
}
#[derive(Component)]
pub struct HerdState {
	pub part: HerdPart,
	pub tick: u32,
	pub ready: bool,
	pub field: Flow,
	slots: Vec<Slot>,
	emitters: Vec<(u32, DistanceEmitter)>,
	fine: Vec<Fine>,
	group: group::Group,
	scene: InteractionScene,
	history: VecDeque<Wake>,
	pub rejected: u64,
	pub accepted: u64,
	pub rebuilds: u64,
	pub geometry_bytes: usize,
	pub max_active: usize,
	pub terrain_misses: u64,
	pub tiles: [Option<[i64; 3]>; 4],
	tile_terrain: [Option<std::sync::Arc<terrain::Heightfield>>; 4],
}
impl HerdState {
	pub fn time(&self) -> f64 {
		self.tick as f64 * DT
	}
	pub fn active(&self) -> usize {
		self.slots.iter().filter(|s| s.puff.is_some()).count()
	}
	pub fn fine_counts(&self) -> (usize, usize) {
		(
			self.fine.iter().filter(|p| !p.grain).count(),
			self.fine.iter().filter(|p| p.grain).count(),
		)
	}
	/// Optional evidence export. Expanded triangles describe the actual closed mesh.
	pub fn snapshot(
		&self,
		meshes: &Assets<Mesh>,
		geometry: bool,
	) -> (String, Vec<(usize, Vec<u8>)>) {
		use std::fmt::Write;
		let mut text = format!(
			"\nHerd tick={} accepted={} rejected={} active={} fine={:?} field_steps={} field_bytes={} group={} representative_height={}\n",
			self.tick,
			self.accepted,
			self.rejected,
			self.active(),
			self.fine_counts(),
			self.field.steps,
			self.field.bytes(),
			self.part.settings.group,
			self.group.representative_height
		);
		let _=writeln!(text,"MovingHerd travelling={} origin={:?} shifts={} terrain_version={} terrain_misses={} optical_tiles={:?}",self.part.settings.travelling,self.field.origin,self.field.shifts,self.terrain_version(),self.terrain_misses,self.tiles);
		for p in &self.fine {
			let _ = writeln!(
				text,
				"HerdFine id={} grain={} age={} position={:?} velocity={:?}",
				p.id,
				p.grain,
				p.age,
				p.position.to_array(),
				p.velocity.to_array()
			);
		}
		let mut output = Vec::new();
		for (id, slot) in self.slots.iter().enumerate() {
			let Some(p) = &slot.puff else {
				continue;
			};
			let b = &p.body;
			let _ = writeln!(
				text,
				"HerdBody slot={id} seed={} birth={} age={} life={} position={:?} velocity={:?} determinant={}",
				p.seed,
				b.birth,
				b.age,
				p.life,
				b.position.to_array(),
				b.velocity.to_array(),
				b.affine.determinant()
			);
			if !geometry {
				continue;
			}
			let Some(mesh) = slot.drawn.as_ref().and_then(|h| meshes.get(h)) else {
				continue;
			};
			let Some(points) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3())
			else {
				continue;
			};
			let indices = mesh
				.indices()
				.map(|v| v.iter().collect::<Vec<_>>())
				.unwrap_or_else(|| (0..points.len()).collect());
			let mut bytes = Vec::with_capacity(indices.len() * 12);
			for i in indices {
				let world = b
					.model()
					.transform_point3(b.pose.coefficients.map(Vec3::from_array(points[i])));
				for v in world.to_array() {
					bytes.extend_from_slice(&v.to_le_bytes());
				}
			}
			output.push((id, bytes));
		}
		for c in self.scene.colliders(self.time()).iter().take(self.scene.collider_count()) {
			let _ = writeln!(
				text,
				"HerdCollider id={} position={:?} velocity={:?} shape={:?} rotation={:?}",
				c.id,
				c.position.to_array(),
				c.linear.to_array(),
				c.shape,
				c.rotation.to_array()
			);
		}
		(text, output)
	}
	pub fn inspection_center(&self) -> Option<Vec3> {
		let mut sum = DVec3::ZERO;
		let mut count = 0;
		for p in self.slots.iter().filter_map(|s| s.puff.as_ref()) {
			sum += p.body.position;
			count += 1;
		}
		(count > 0).then(|| (sum / count as f64).as_vec3())
	}
	pub fn volume_inspection_point(&self) -> Option<Vec3> {
		self.slots
			.iter()
			.filter_map(|s| s.puff.as_ref())
			.find(|p| p.style == Style::Combined)
			.map(|p| p.body.position.as_vec3())
	}
	pub(crate) fn has_optical(&self) -> bool {
		self.fine_counts() != (0, 0)
			|| self
				.slots
				.iter()
				.filter_map(|s| s.puff.as_ref())
				.any(|p| p.style == Style::Combined)
	}
	pub(crate) fn optical_colliders(&self) -> Vec<Interactor> {
		self.scene
			.colliders(self.time())
			.into_iter()
			.take(self.scene.collider_count())
			.collect()
	}
	pub fn optical_primitives(&self, supports: [(Vec3, Vec3); 3]) -> Vec<Primitive> {
		let mut bodies = Vec::with_capacity(MAX_PUFFS + MAX_FINE + MAX_GRAINS);
		for p in self
			.slots
			.iter()
			.filter_map(|s| s.puff.as_ref())
			.filter(|p| p.style == Style::Combined)
		{
			let b = &p.body;
			let bounds = supports[b.pose.variant];
			let (lo, hi) = b.pose.bounds(bounds.0, bounds.1);
			let environmental = b.model() * b.pose.transform().to_matrix().inverse();
			let (min, max) = Primitive::transformed_bounds(environmental, lo, hi);
			let end = ((1. - b.age / p.life) / 0.35).clamp(0., 1.) as f32;
			bodies.push(Primitive {
				inverse: b.model().inverse(),
				masked: true,
				coefficients: b.pose.coefficients.0,
				parameters: Vec4::new(
					b.pose.variant as f32,
					b.phase as f32,
					b.concentration as f32
						* self.part.settings.body_density
						* end * end * (3. - 2. * end),
					0.,
				),
				min,
				max,
			});
		}
		bodies.extend(self.fine.iter().map(|p| {
			let f = (p.age / p.life).clamp(0., 1.);
			let radius = if p.grain { p.size } else { p.size * (0.65 + f) };
			let model = Mat4::from_scale_rotation_translation(
				Vec3::splat(radius as f32),
				Quat::from_rotation_y(p.seed as f32),
				p.position.as_vec3(),
			);
			let (min, max) = Primitive::transformed_bounds(model, -Vec3::ONE, Vec3::ONE);
			Primitive {
				inverse: model.inverse(),
				masked: true,
				coefficients: Vec4::ZERO,
				parameters: if p.grain {
					Vec4::new(0.55, 0.32, 0.13, 2.)
				} else {
					Vec4::new(
						(3 + p.seed % 4) as f32,
						0.,
						((1. - f) * 0.22) as f32 * self.part.settings.fine_density,
						1.,
					)
				},
				min,
				max,
			}
		}));
		bodies
	}
	pub fn terrain_version(&self) -> u64 {
		self.scene.terrain.as_ref().map_or(0, |h| h.version)
	}
	pub fn terrain_data(&self) -> Option<&terrain::Heightfield> {
		self.scene.terrain.as_deref()
	}
	pub fn optical_terrain(&self, tile: usize) -> Option<&terrain::Heightfield> {
		self.tile_terrain[tile].as_deref()
	}
	pub fn optical_tile(&self, tile: usize, supports: [(Vec3, Vec3); 3]) -> Vec<Primitive> {
		let ids = self
			.slots
			.iter()
			.filter_map(|s| s.puff.as_ref())
			.filter(|p| p.style == Style::Combined)
			.map(|p| p.tile)
			.chain(self.fine.iter().map(|p| p.tile));
		self.optical_primitives(supports)
			.into_iter()
			.zip(ids)
			.filter_map(|(p, id)| (id == tile).then_some(p))
			.collect()
	}
	/// Host coverage includes current flow AND every live released support/particle.
	pub fn terrain_requirements(&self) -> (DVec3, DVec3) {
		let mut lo = self.field.origin;
		let mut hi = lo + DVec3::new(6., 3., 6.);
		for p in self.slots.iter().filter_map(|s| s.puff.as_ref()) {
			let r = p.body.scale.max_element() * 4.;
			lo = lo.min(p.body.position - DVec3::splat(r));
			hi = hi.max(p.body.position + DVec3::splat(r));
		}
		for p in &self.fine {
			// Fine density expands to1.65× size and its rotated cube support needs sqrt(2).
			let r = p.size * 2.5;
			lo = lo.min(p.position - DVec3::splat(r));
			hi = hi.max(p.position + DVec3::splat(r));
		}
		(lo, hi)
	}

	fn birth(
		&mut self,
		birth: Birth,
		style: Style,
		fields: &ClosedFields,
		factor: f64,
		formation: Option<(f64, f64)>,
	) -> bool {
		if style == Style::None {
			return false;
		}
		if (!self.part.settings.travelling && !in_domain(birth.position, 0.3))
			|| (self.part.settings.travelling
				&& (!self.field.covers(birth.position, 0.15)
					|| self
						.scene
						.terrain
						.as_ref()
						.and_then(|h| h.sample(birth.position))
						.is_none()))
		{
			self.rejected += 1;
			return false;
		}
		let tile = if self.part.settings.travelling && style == Style::Combined {
			let key = (birth.position / 4.).floor().to_array().map(|v| v as i64);
			if let Some(i) = self.tiles.iter().position(|t| *t == Some(key)) {
				i
			} else if let Some(i) = self.tiles.iter().position(Option::is_none) {
				self.tiles[i] = Some(key);
				i
			} else {
				self.rejected += 1;
				return false;
			}
		} else {
			0
		};
		let ground = self.scene.terrain.as_ref().and_then(|h| h.sample(birth.position));
		let ground_cache = self.scene.terrain.clone();
		if self.part.settings.travelling
			&& style == Style::Combined
			&& self.tile_terrain[tile].is_none()
		{
			self.tile_terrain[tile] = ground_cache.clone();
		}
		let Some(slot) = self.slots.iter_mut().find(|s| s.puff.is_none()) else {
			self.rejected += 1;
			return false;
		};
		let random = |salt| (mix32(birth.seed ^ salt) & 0xffffff) as f64 / 16777216.;
		let life = self.part.settings.lifetime.clamp(0.3, 2.)
			* if birth.seed % 9 == 0 { 1.8 } else { 0.85 + random(1) * 0.3 };
		let size = factor * self.part.settings.size.clamp(0.06, 0.35) * (0.85 + random(2) * 0.3);
		let aspect = if formation.is_some() { 0.85 } else { 1. };
		let (size, life) = formation.unwrap_or((size, life));
		let normal = birth.normal.normalize_or(DVec3::Y);
		let rotation = Quat::from_rotation_arc(Vec3::Y, normal.as_vec3())
			* Quat::from_rotation_y((random(3) * 6.28) as f32);
		let pose = Pose {
			position: birth.position.as_vec3(),
			rotation,
			scale: Vec3::new(size as f32, size as f32 * aspect as f32, size as f32) * 0.6,
			pivot: Vec3::ZERO,
			coefficients: Deformation::at(0., 0),
			variant: birth.seed as usize % 3,
			phase: 0.,
			clip: 0,
			birth: birth.time as f32,
			yaw: 0.,
			support: 0.,
			tilt: 0.,
		};
		let direction = (birth.velocity * DVec3::new(1., 0., 1.)).normalize_or(DVec3::Z);
		let kick = (-direction * 0.14
			+ normal.cross(direction) * (random(4) - 0.5) * 0.35
			+ normal * 0.16)
			* self.part.settings.strength.clamp(0., 2.);
		let kick = if formation.is_some() {
			(-direction * 0.08 + normal * 0.06) * (size / 0.48)
		} else {
			kick
		};
		let centre = pose.world(Vec3::new(0.05, 0.66, 0.)).as_dvec3();
		let body = BodyMotion {
			flow_guided: true,
			affine: Mat3::IDENTITY,
			previous_model: pose.transform().to_matrix(),
			id: birth.seed,
			position: centre,
			velocity: birth.velocity * 0.07 + kick,
			rotation: rotation.as_dquat(),
			scale: pose.scale.as_dvec3(),
			age: 0.,
			phase: random(5) * 2.,
			birth: birth.time,
			pose,
			previous: pose,
			concentration: 1.,
		};
		let f = &fields.fields[pose.variant];
		f.fill_noise(birth.seed, &mut slot.noise);
		slot.puff = Some(Puff {
			aspect,
			body,
			life,
			size,
			normal,
			style,
			side: 0.,
			release: 0,
			seed: birth.seed,
			tile,
			ground,
			ground_cache,
		});
		self.accepted += 1;
		true
	}
	fn step(
		&mut self,
		mut frame: Frame,
		fields: &ClosedFields,
		seed: u32,
		mut profile: Option<&mut Probe>,
	) {
		if self.part.settings.travelling {
			let h = self.scene.terrain.clone();
			for e in &mut frame.emitters {
				if let Some(g) = h.as_ref().and_then(|h| h.sample(e.position)) {
					e.position = g.point;
					e.ground.normal = g.normal;
				} else {
					e.ground.grounded = false;
					self.terrain_misses += 1;
				}
			}
			if !frame.emitters.is_empty() && self.tick % 2 == 0 {
				let center = frame.emitters.iter().map(|e| e.position).sum::<DVec3>()
					/ frame.emitters.len() as f64;
				let low = frame.emitters.iter().map(|e| e.position.y).fold(center.y, f64::min);
				let desired =
					(DVec3::new(center.x - 3., low - 0.5, center.z - 3.) / 0.25).floor() * 0.25;
				let mid = self.field.origin + DVec3::new(3., 0.5, 3.);
				if self.tick == 0 || (center - mid).abs().max_element() > 0.75 {
					self.field.shift_to(desired);
				}
			}
		}
		let formations = if self.part.settings.group {
			self.group.sample_with_bounds(
				frame.time,
				&frame.emitters,
				&frame.colliders,
				&frame.heights,
				seed,
				frame.emitting,
				self.part.settings.coverage,
				!self.part.settings.travelling,
			)
		} else {
			Vec::new()
		};

		let t = self.time();
		// Flow culls per-cell conservatively; retain all inputs for old dust/contact/occupancy.
		self.scene.external = Some(frame.colliders);
		self.scene.changed();
		if self.tick % 2 == 0
			&& (frame.style != Style::None || self.active() > 0 || !self.fine.is_empty())
		{
			timed!(profile, Flow, self.field.step(&self.scene, t));
			if let Some(p) = profile.as_deref_mut() {
				p.frame.flow_steps += 1;
			}
		}
		self.history.retain(|w| t - w.birth < 1.6);
		if self.tick % 10 == 0 {
			for c in self.scene.colliders(t).iter().take(self.scene.collider_count()) {
				if c.linear.length_squared() > 0.01 {
					self.history.push_back(Wake {
						position: c.position,
						velocity: c.linear,
						birth: t,
					});
				}
			}
		}
		let mut births = if frame.emitting { frame.births } else { Vec::new() };
		self.emitters.retain(|(id, _)| frame.emitters.iter().any(|e| e.id == *id));
		for input in frame.emitters {
			if !self.emitters.iter().any(|(id, _)| *id == input.id) && self.emitters.len() < 20 {
				self.emitters.push((input.id, DistanceEmitter::new(seed, input.id)));
			}
			if let Some((_, e)) = self.emitters.iter_mut().find(|(id, _)| *id == input.id) {
				let mut b = Vec::new();
				e.sample(frame.time, input, self.part.settings.spacing, &mut b);
				if frame.emitting && !self.part.settings.group {
					births.extend(b);
				}
			}
		}
		births.sort_by(|a, b| a.time.total_cmp(&b.time).then(a.seed.cmp(&b.seed)));

		for mut f in formations {
			if self.part.settings.travelling {
				if let Some(g) =
					self.scene.terrain.as_ref().and_then(|h| h.sample(f.birth.position))
				{
					f.birth.position = g.point + g.normal * 0.012;
					f.birth.normal = g.normal;
				} else {
					self.rejected += 1;
					continue;
				}
				let q = f.birth.position - self.field.origin;
				let margin = f.size * 1.6;
				if q.x < margin
					|| q.z < margin || q.x > 6. - margin
					|| q.z > 6. - margin
					|| q.y < 0. || q.y + f.size * 3.5 > 3.
				{
					self.rejected += 1;
					continue;
				}
			}
			self.birth(f.birth, frame.style, fields, 1., Some((f.size, f.life)));
		}
		let mut companions = Vec::with_capacity(4);
		for birth in births {
			if self.birth(birth, frame.style, fields, 1., None) && birth.seed % 7 == 0 {
				companions.push(birth);
			}
		}
		// Main requests have priority. Companion rejection never removes a visible puff.
		for mut b in companions {
			b.seed = mix32(b.seed ^ 0x9187);
			b.position += b.normal.cross(b.velocity).normalize_or(DVec3::X) * 0.10;
			self.birth(b, frame.style, fields, 0.6, None);
		}

		for slot in &mut self.slots {
			let Some(p) = &mut slot.puff else {
				continue;
			};
			p.body.age = (frame.time - p.body.birth).max(0.);
			if p.body.age >= p.life {
				slot.puff = None;
				continue;
			}
			let b = &mut p.body;
			b.previous = b.pose;
			b.previous_model = b.model();
			b.advect(&self.field, &self.scene, &self.history, t);

			// Select nearest local contact for the approved compression/turn helper;
			// contour and flow still see every relevant collider.
			if let Some(c) =
				self.scene.colliders(t).into_iter().take(self.scene.collider_count()).min_by(
					|a, c| a.query(b.position).distance.total_cmp(&c.query(b.position).distance),
				) {
				let mut local = self.scene.clone();
				local.scenario = crate::parts::combined::interaction::Scenario::Manual;
				local.manual.clear();
				local.place(t, c.position, true);
				// Helper uses capsule(); preserve this proxy's dimensions via external primary.
				local.external = Some(vec![c]);
				crate::parts::cartoon_flow::contact::respond(b, &mut p.side, &local, t);
			}
			b.phase += DT * 1.8;
			let growth = 0.6 + 0.4 * (1. - (-b.age * 12.).exp());
			b.scale = DVec3::new(p.size, p.size * p.aspect, p.size) * growth;

			b.update_pose();
			let f = &fields.fields[b.pose.variant];
			let model = b.model();
			if self.part.settings.travelling {
				if b.age > 0.15 {
					b.velocity.y = (b.velocity.y - self.part.settings.settling_accel * DT)
						.max(-self.part.settings.settling_speed);
				}
				if let Some(h) = &self.scene.terrain {
					if f.support.iter().all(|r| {
						h.sample(model.transform_point3(b.pose.coefficients.map(*r)).as_dvec3())
							.is_some()
					}) {
						p.ground_cache = Some(h.clone());
					}
				}
				let mut lift = 0.;
				let mut normal = DVec3::Y;
				for rest in f.support.iter() {
					let w = model.transform_point3(b.pose.coefficients.map(*rest)).as_dvec3();
					let ground = self
						.scene
						.terrain
						.as_ref()
						.and_then(|h| h.sample(w))
						.or_else(|| p.ground_cache.as_ref().and_then(|h| h.sample(w)));
					if let Some(g) = ground {
						p.ground = Some(g);
						let d = g.point.y + 0.012 - w.y;
						if d > lift {
							lift = d;
							normal = g.normal;
						}
					} else {
						self.terrain_misses += 1;
						if let Some(g) = p.ground {
							let d = g.point.y + 0.012 - w.y;
							if d > lift {
								lift = d;
								normal = DVec3::Y;
							}
						}
					}
				}
				if lift > 0. {
					b.position.y += lift;
					b.velocity -= normal * b.velocity.dot(normal).min(0.);
					b.update_pose();
				}
			} else {
				let floor = f
					.support
					.iter()
					.map(|r| model.transform_point3(b.pose.coefficients.map(*r)).y)
					.fold(f32::INFINITY, f32::min);
				if floor < 0.012 {
					b.position.y += (0.012 - floor) as f64;
					b.velocity.y = b.velocity.y.max(0.);
					b.update_pose();
				}
			}

			if p.style == Style::Combined
				&& p.release < 2
				&& b.age > 0.10 + f64::from(p.release) * 0.17
			{
				p.release += 1;
				let rest = Vec3::new(if p.seed & 1 == 0 { 0.4 } else { -0.4 }, 0.55, 0.);
				let rest = if p.aspect < 1. {
					f.support[(p.seed as usize + p.release as usize * 17) % f.support.len()]
				} else {
					rest
				};
				let (mut position, motion) = b.material_point(rest);
				if p.aspect < 1. {
					position += (position - b.position).normalize_or(DVec3::Y) * p.size * 0.06;
				}
				for grain in [false, true] {
					let max = if grain {
						self.part.settings.max_grains
					} else {
						self.part.settings.max_fine
					};
					if self.fine.iter().filter(|p| p.grain == grain).count() >= max {
						continue;
					}
					self.fine.push(Fine {
						id: ((p.seed as u64) << 8) | ((p.release as u64) << 1) | grain as u64,
						position,
						velocity: self.field.sample(position)
							+ motion.clamp_length_max(1.) * 0.18
							+ p.normal * 0.06,
						age: 0.,
						life: if grain { 0.65 } else { 1.1 },
						size: if p.aspect < 1. {
							if grain {
								p.size * 0.05
							} else {
								p.size * 0.65
							}
						} else if grain {
							0.009
						} else {
							0.12
						},
						grain,
						seed: p.seed,
						tile: p.tile,
						ground: p.ground,
						ground_cache: p.ground_cache.clone(),
					});
				}
			}
		}
		for p in &mut self.fine {
			let target = self.field.sample(p.position);
			let (position, velocity) = self.scene.integrate(
				p.position,
				p.velocity,
				target,
				if p.grain { 1.5 } else { 4. },
				if p.grain { 0.5 } else { 0. },
				if p.grain { p.size } else { 0. },
				t,
			);
			p.position = position;
			p.velocity = velocity;
			if self.part.settings.travelling {
				if let Some(h) = &self.scene.terrain {
					if h.sample(p.position).is_some() {
						p.ground_cache = Some(h.clone());
					}
				}
				let g = self
					.scene
					.terrain
					.as_ref()
					.and_then(|h| h.sample(p.position))
					.or_else(|| p.ground_cache.as_ref().and_then(|h| h.sample(p.position)));
				if let Some(g) = g {
					p.ground = Some(g);
				} else {
					self.terrain_misses += 1;
				}
				if !p.grain {
					p.velocity.y = (p.velocity.y - self.part.settings.settling_accel * DT)
						.max(-self.part.settings.settling_speed);
				}
				if let Some(g) = p.ground {
					let extent = if p.grain { p.size } else { p.size * 0.15 };
					let d = (p.position - g.point).dot(g.normal) - extent;
					if d < 0. {
						p.position -= g.normal * d;
						p.velocity -= g.normal * p.velocity.dot(g.normal).min(0.);
					}
				}
			}
			p.age += DT;
		}
		self.fine.retain(|p| p.age < p.life);
		for i in 0..4 {
			if !self
				.slots
				.iter()
				.filter_map(|s| s.puff.as_ref())
				.any(|p| p.style == Style::Combined && p.tile == i)
				&& !self.fine.iter().any(|p| p.tile == i)
			{
				self.tiles[i] = None;
				self.tile_terrain[i] = None;
			}
		}
		// Refresh retained optical coverage only when the new snapshot covers every
		// member support; otherwise keep the old world patch until normal retirement.
		if let Some(h) = &self.scene.terrain {
			for tile in 0..4 {
				if self.tiles[tile].is_none() {
					continue;
				}
				let bodies = self
					.slots
					.iter()
					.filter_map(|s| s.puff.as_ref())
					.filter(|p| p.style == Style::Combined && p.tile == tile)
					.all(|p| {
						fields.fields[p.body.pose.variant].support.iter().all(|r| {
							h.sample(
								p.body
									.model()
									.transform_point3(p.body.pose.coefficients.map(*r))
									.as_dvec3(),
							)
							.is_some()
						})
					});
				if bodies
					&& self
						.fine
						.iter()
						.filter(|p| p.tile == tile)
						.all(|p| h.sample(p.position).is_some())
				{
					self.tile_terrain[tile] = Some(h.clone());
				}
			}
		}
		self.tick += 1;
		self.max_active = self.max_active.max(self.active());
	}
}
fn in_domain(p: DVec3, margin: f64) -> bool {
	p.x >= -2. + margin
		&& p.x <= 4. - margin
		&& p.z >= -3. + margin
		&& p.z <= 3. - margin
		&& p.y >= 0.
		&& p.y < 3.
}
impl HerdPart {
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) {
		if let Err(e) = self.settings.validate() {
			warn!("{e}");
			commands.entity(parent).try_despawn();
			return;
		}
		let scene = InteractionScene {
			external: Some(Vec::new()),
			wind: DVec3::new(0.06, 0., 0.),
			..default()
		};
		commands.spawn((
			ChildOf(parent),
			VfxMemberOf(parent),
			transform,
			Visibility::Visible,
			VfxLayerLife {
				age: 0.,
				duration: f32::MAX,
				playback: spawn.clamped_playback(),
				waiting_for_emitter: true,
			},
			HerdState {
				part: self.clone(),
				tick: 0,
				ready: false,
				field: Flow::new(scene.wind),
				slots: Vec::new(),
				emitters: Vec::with_capacity(20),
				fine: Vec::with_capacity(MAX_FINE + MAX_GRAINS),
				scene,
				history: VecDeque::with_capacity(400),
				group: default(),
				rejected: 0,
				accepted: 0,
				rebuilds: 0,
				geometry_bytes: 0,
				max_active: 0,
				terrain_misses: 0,
				tiles: [None; 4],
				tile_terrain: std::array::from_fn(|_| None),
			},
		));
	}
}
pub struct HerdPlugin;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HerdTick;
impl Plugin for HerdPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<terrain::HerdBudget>()
			.add_systems(Update, tick.in_set(HerdTick).after(crate::lifecycle::tick_vfx_instances));
	}
}
fn tick(
	mut commands: Commands,
	mut probe: Option<ResMut<Probe>>,
	mut fields: ResMut<ClosedFields>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<HerdMaterial>>,
	clock: Res<Time<Virtual>>,
	instances: Query<
		(&VfxInstance, Option<&terrain::HerdTerrain>, &Transform, Option<&ChildOf>),
		Without<Mesh3d>,
	>,
	budget: Res<terrain::HerdBudget>,
	mut inputs: Query<&mut Inputs>,
	mut states: Query<(Entity, &VfxMemberOf, &mut HerdState, &mut VfxLayerLife)>,
	mut visuals: Query<(&Mesh3d, &mut Transform, &mut Visibility, &mut Aabb)>,
) {
	let mut profile = probe.as_deref_mut();
	let total_stamp = profile.as_ref().map(|_| std::time::Instant::now());
	let mut ids = states.iter().map(|(e, _, s, _)| (!s.ready, e)).collect::<Vec<_>>();
	ids.sort();
	let existing = ids.iter().filter(|(new, _)| !new).count();
	let mut remaining = budget.maximum.saturating_sub(existing);
	let admitted = ids
		.into_iter()
		.filter_map(|(new, e)| {
			if !new {
				Some(e)
			} else if remaining > 0 {
				remaining -= 1;
				Some(e)
			} else {
				None
			}
		})
		.collect::<Vec<_>>();
	for (entity, member, mut s, mut life) in &mut states {
		if !admitted.contains(&entity) {
			warn!("Herd admission budget exceeded");
			commands.entity(member.root()).try_despawn();
			continue;
		}
		let Ok((instance, terrain, root_transform, parent)) = instances.get(member.root()) else {
			continue;
		};
		if parent.is_some() || !root_transform.to_matrix().abs_diff_eq(Mat4::IDENTITY, 1e-5) {
			warn!("Herd roots must remain identity; world placement comes from inputs");
			commands.entity(member.root()).try_despawn();
			continue;
		}
		if s.part.settings.travelling {
			s.scene.terrain = terrain.map(|h| h.0.clone());
			s.scene.ground_floor = None;
			s.field.rolling = true;
		}
		if !fields.prepare(&mut meshes, &s.part.meshes) {
			continue;
		}
		if !s.ready {
			for _ in 0..s.part.settings.max_puffs.clamp(1, MAX_PUFFS) {
				let mesh = meshes.reserve_handle();
				let material = materials.add(HerdMaterial::wave(Vec4::ZERO));
				let id = commands
					.spawn((
						ChildOf(entity),
						VfxMemberOf(member.root()),
						Mesh3d(fields.fields[0].base.clone()),
						MeshMaterial3d(material.clone()),
						Transform::IDENTITY,
						Visibility::Hidden,
						NoAutoAabb,
						Aabb::from_min_max(Vec3::splat(-3.), Vec3::splat(3.)),
					))
					.id();
				s.slots.push(Slot {
					entity: id,
					material,
					mesh,
					noise: Vec::with_capacity(8000),
					nodes: Vec::with_capacity(8000),
					drawn: None,
					puff: None,
				});
			}
			s.ready = true;
			life.waiting_for_emitter = false;
		}
		let Ok(mut input) = inputs.get_mut(member.root()) else {
			continue;
		};
		let target = (instance.age as f64 / DT + 1e-5).floor() as u32;
		let before = s.tick;
		if instance.armed && !clock.is_paused() {
			while s.tick < target {
				let Some(frame) = input.frames.pop_front() else {
					break;
				};
				if (frame.time - (s.time() + DT)).abs() > 1e-5 {
					warn!("Herd input gap; refusing to advance");
					break;
				}
				s.step(frame, &fields, instance.seed as u32, profile.as_deref_mut());
				if let Some(p) = profile.as_deref_mut() {
					p.frame.ticks += 1;
				}
			}
		}
		life.age = s.time() as f32;
		if before == s.tick {
			continue;
		}
		let time = s.time();
		let scene = s.scene.clone();
		let mut rebuilds = 0;
		let mut bytes = 0;
		let travelling = s.part.settings.travelling;
		for slot in &mut s.slots {
			let Ok((current, mut transform, mut visible, mut bounds)) =
				visuals.get_mut(slot.entity)
			else {
				continue;
			};
			slot.drawn = None;
			let Some(p) = &mut slot.puff else {
				*visible = Visibility::Hidden;
				continue;
			};
			if p.style == Style::Combined {
				*visible = Visibility::Hidden;
				continue;
			}
			let b = &p.body;
			let f = &fields.fields[b.pose.variant];
			let fraction = b.age / p.life;
			let threshold =
				if fraction < 0.65 { -1. } else { ((fraction - 0.65) / 0.35 * 1.03) as f32 };
			let near = f.needs_contact(b, &scene, time) || scene.terrain.is_some();
			if near {
				if let Some(p) = profile.as_deref_mut() {
					p.frame.near_solid += 1;
				}
			}
			*visible = Visibility::Visible;
			if near || threshold >= 0. {
				slot.nodes.clear();
				slot.nodes.extend(f.nodes.iter().map(|r| b.pose.coefficients.map(*r)));
				// Conservative per-body collider selection; never omit a nearby solid.
				let mut local = scene.clone();
				if travelling {
					local.terrain = p.ground_cache.clone();
				}
				let (lo, hi) = f.bounds_for(b.pose);
				let affine = b.model() * b.pose.transform().to_matrix().inverse();
				let (lo, hi) = Primitive::transformed_bounds(affine, lo, hi);
				let center = ((lo + hi) * 0.5).as_dvec3();
				let radius = (hi - lo).length() as f64 * 0.5;
				if let Some(colliders) = &mut local.external {
					colliders.retain(|c| c.query(center).distance < radius + 0.05);
				}
				let generated = f.rebuild_profiled(
					b,
					&local,
					time,
					threshold,
					Some(&slot.noise),
					&slot.nodes,
					profile.as_deref_mut(),
				);
				rebuilds += 1;
				bytes += generated.count_vertices() * 24
					+ generated.indices().map_or(0, |i| i.len() * 4);
				if generated.count_vertices() == 0 {
					*visible = Visibility::Hidden;
					continue;
				}
				if meshes.insert(slot.mesh.id(), generated).is_err() {
					*visible = Visibility::Hidden;
					continue;
				}
				slot.drawn = Some(slot.mesh.clone());
				if current.0 != slot.mesh {
					commands.entity(slot.entity).try_insert(Mesh3d(slot.mesh.clone()));
				}
			} else {
				slot.drawn = Some(f.base.clone());
				if current.0 != f.base {
					commands.entity(slot.entity).try_insert(Mesh3d(f.base.clone()));
				}
			}
			*transform = b.pose.transform();
			let original = transform.to_matrix();
			let map = b.model() * original.inverse();
			let material = HerdMaterial {
				cutoff: b.pose.coefficients.0,
				seed: UVec4::new(p.seed, 0, 2, 1),
				environment: SurfaceEnvironment {
					map,
					normal: map.inverse().transpose(),
					..default()
				},
			};
			if let Some(current) = materials.get(&slot.material) {
				if current.cutoff != material.cutoff
					|| current.seed != material.seed
					|| current.environment != material.environment
				{
					if let Some(mut m) = materials.get_mut(&slot.material) {
						*m = material;
					}
				}
			}
			let correction = original.inverse() * map * original;
			let (lo, hi) = f.bounds_for(Pose {
				position: Vec3::ZERO,
				rotation: Quat::IDENTITY,
				scale: Vec3::ONE,
				pivot: Vec3::ZERO,
				..b.pose
			});
			let (lo, hi) = Primitive::transformed_bounds(correction, lo, hi);
			*bounds = Aabb::from_min_max(lo, hi);
		}
		s.rebuilds += rebuilds;
		s.geometry_bytes = bytes;
		if let Some(p) = profile.as_deref_mut() {
			p.frame.rebuilds += rebuilds as u32;
			p.frame.upload_bytes += bytes;
			p.frame.bodies = s.active();
			p.frame.tick = s.tick;
			p.frame.age = time as f32;
			p.frame.debt = target.saturating_sub(s.tick);
		}
	}
	if let (Some(p), Some(stamp)) = (profile, total_stamp) {
		p.frame.ns[0] += stamp.elapsed().as_nanos() as u64;
	}
}

#[cfg(test)]
mod tests;
