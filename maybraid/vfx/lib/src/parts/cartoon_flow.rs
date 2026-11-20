pub mod contact;
pub mod diagnostics;
pub mod region;
use crate::parts::cartoon_flow::diagnostics::{timed, Mode, Probe};
mod distance;
pub(crate) mod surface;
// Opaque authored bodies driven by the shared body controller. No secondary pools.
use crate::{
	composition::CartoonFlowPart,
	membership::VfxMemberOf,
	parts::{
		body_flow::BodyMotion,
		cartoon_landing::{motion, LandingMaterial},
		cartoon_wave::{
			motion::{Deformation, Pose, Wave},
			Geometry,
		},
		combined::{
			flow::Flow,
			interaction::{InteractionScene, Wake, DT, MAX_WAKES},
		},
	},
	spawn::{VfxInstance, VfxLayerLife, VfxSpawn},
};
use bevy::{
	camera::primitives::Aabb, math::DVec3, prelude::*, render::render_resource::ShaderType,
};
use std::{collections::VecDeque, sync::Arc};
#[derive(Clone, Debug, PartialEq, ShaderType)]
pub struct SurfaceEnvironment {
	pub map: Mat4,
	pub normal: Mat4,
	pub colliders: [Vec4; 12],
	pub settings: Vec4,
}
impl Default for SurfaceEnvironment {
	fn default() -> Self {
		Self {
			map: Mat4::IDENTITY,
			normal: Mat4::IDENTITY,
			colliders: [Vec4::ZERO; 12],
			settings: Vec4::ZERO,
		}
	}
}
impl SurfaceEnvironment {
	fn new(map: Mat4, scene: &InteractionScene, t: f64) -> Self {
		let mut s = Self { map, normal: map.inverse().transpose(), ..default() };
		s.settings.x = scene.collider_count().min(3) as f32;
		for (i, c) in scene.colliders(t).iter().take(scene.collider_count().min(3)).enumerate() {
			let m = Mat4::from_rotation_translation(c.rotation.as_quat(), c.position.as_vec3())
				.inverse();
			s.colliders[i * 4] = m.row(0);
			s.colliders[i * 4 + 1] = m.row(1);
			s.colliders[i * 4 + 2] = m.row(2);
			s.colliders[i * 4 + 3] = match c.shape {
				crate::parts::combined::interaction::Shape::Capsule { radius, half_segment } => {
					Vec4::new(radius as f32, half_segment as f32, 0., 0.)
				}
				crate::parts::combined::interaction::Shape::Box { half } => {
					half.as_vec3().extend(1.)
				}
			};
		}
		s
	}
}
#[derive(Component)]
pub struct CartoonFlow {
	pub landing: bool,
	pub contact: bool,
	pub region: Option<region::ContactRegion>,
	pub tick: u32,
	part: CartoonFlowPart,
	wave: Option<Wave>,
	rest: Option<[Arc<[Vec3]>; 3]>,
	specs: Vec<motion::Puff>,
	pub bodies: Vec<Option<BodyMotion>>,
	pub field: Option<Flow>,
	history: VecDeque<Wake>,
	pub ready: bool,
	surface_nodes: [Vec<Vec3>; 16],
	surface_ticks: [u32; 16],
	surface_noise: [Vec<f32>; 16],
	contact_sides: [f64; 48],
	ground_cache: [Vec<Vec3>; 16],
}
#[derive(Component)]
pub struct Surface {
	pub index: usize,
	pub controller: Entity,
	material: Handle<LandingMaterial>,
	dynamic: Option<Handle<Mesh>>,
	geometry_tick: u32,
	geometry_scene: u64,
	geometry_empty: bool,
}
impl Surface {
	pub fn has_surface(&self) -> bool {
		!self.geometry_empty
	}
}

impl CartoonFlowPart {
	pub(crate) fn spawn(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
	) -> Entity {
		self.spawn_controller(commands, parent, transform, spawn, true)
	}
	pub(crate) fn spawn_controller(
		&self,
		commands: &mut Commands,
		parent: Entity,
		transform: Transform,
		spawn: &VfxSpawn,
		contact: bool,
	) -> Entity {
		commands
			.spawn((
				ChildOf(parent),
				VfxMemberOf(parent),
				transform,
				Visibility::Visible,
				VfxLayerLife {
					age: 0.,
					duration: if self.landing { 1.2 } else { 4. },
					playback: spawn.clamped_playback(),
					waiting_for_emitter: true,
				},
				CartoonFlow {
					landing: self.landing,
					contact,
					region: None,
					tick: 0,
					part: self.clone(),
					wave: None,
					rest: None,
					specs: Vec::new(),
					bodies: vec![None; if self.refined { 48 } else { 16 }],
					field: None,
					history: VecDeque::with_capacity(MAX_WAKES),
					ready: false,
					surface_nodes: std::array::from_fn(|_| Vec::new()),
					surface_ticks: [u32::MAX; 16],
					surface_noise: std::array::from_fn(|_| Vec::new()),
					contact_sides: [0.; 48],
					ground_cache: std::array::from_fn(|_| Vec::new()),
				},
			))
			.id()
	}
}
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CartoonFlowTick;
pub struct CartoonFlowPlugin;
impl Plugin for CartoonFlowPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<region::ContactRegionBudget>()
			.init_resource::<surface::ClosedFields>()
			.add_systems(
				Update,
				tick.in_set(CartoonFlowTick).after(crate::lifecycle::tick_vfx_instances),
			)
			.add_systems(
				PostUpdate,
				validate_region_frames
					.after(TransformSystems::Propagate)
					.before(crate::parts::combined::render::compose),
			);
	}
}
/// Catch root/ancestor edits after Update too, before any changed hierarchy renders.
fn validate_region_frames(
	mut commands: Commands,
	regions: Query<(&VfxMemberOf, &GlobalTransform, &CartoonFlow)>,
) {
	for (member, global, state) in &regions {
		if state.ready
			&& state
				.region
				.as_ref()
				.is_some_and(|r| !r.frame.world_from_local.abs_diff_eq(global.to_matrix(), 1e-5))
		{
			warn!(
				"Contact region placement changed after admission; cancelling {:?}",
				member.root()
			);
			commands.entity(member.root()).try_despawn();
		}
	}
}

impl CartoonFlow {
	pub fn retirement(&self, id: usize) -> (f32, u32) {
		self.authored(self.tick as f64 * DT, id).map_or((-1., 0), |(_, t, s)| (t, s))
	}
	fn authored_base(&self, t: f64, index: usize) -> Option<(Pose, f32, u32)> {
		if self.landing {
			let spec = self.specs.get(index)?;
			let s = spec.state(t)?;
			Some((
				Pose {
					position: s.transform.translation,
					rotation: s.transform.rotation,
					scale: s.transform.scale,
					pivot: Vec3::ZERO,
					coefficients: Deformation(Vec4::ZERO),
					variant: spec.variant,
					phase: 0.,
					clip: 0,
					birth: spec.birth as f32,
					yaw: 0.,
					support: 0.,
					tilt: 0.,
				},
				s.threshold,
				spec.seed,
			))
		} else {
			let w = self.wave.as_ref()?;
			let p =
				if index < 12 { w.parent(t as f32, index) } else { w.child(t as f32, index - 12) }?;
			Some((p, 0., 0))
		}
	}
	fn count(&self) -> usize {
		(if self.landing { 16 } else { 15 }) * if self.part.refined { 3 } else { 1 }
	}
	fn authored(&self, t: f64, index: usize) -> Option<(Pose, f32, u32)> {
		if !self.part.refined {
			return self.authored_base(t, index);
		}
		let (mut p, threshold, seed) = if !self.landing && self.bodies[index].is_some() {
			(self.wave.as_ref()?.released_shape(t as f32, index / 3)?, 0., 0)
		} else {
			self.authored_base(t, index / 3)?
		};
		let (offset, factor) = match index % 3 {
			0 => (Vec3::new(-0.14, 0.08, -0.04), 0.76),
			1 => (Vec3::new(0.36, 0.04, 0.25), 0.58),
			_ => (Vec3::new(-0.05, 0.28, -0.36), 0.49),
		};
		let center = Vec3::new(0.05, 0.66, 0.);
		let world = p.world(center);
		p.scale *= factor;
		p.position = world + p.rotation * (offset * p.scale / factor)
			- p.rotation * (p.coefficients.map(center) * p.scale);
		p.pivot = Vec3::ZERO;
		Some((p, threshold, seed))
	}

	fn world_pose(p: Pose, parent: Mat4) -> Pose {
		let (s, r, t) = (parent * p.transform().to_matrix()).to_scale_rotation_translation();
		Pose { position: t, rotation: r, scale: s, pivot: Vec3::ZERO, ..p }
	}
	fn initialize(id: usize, p: Pose, v: DVec3) -> BodyMotion {
		let position = p.world(Vec3::new(0.05, 0.66, 0.)).as_dvec3();
		let mut b = BodyMotion {
			flow_guided: true,
			id: id as u32,
			position,
			velocity: v,
			rotation: p.rotation.as_dquat(),
			scale: p.scale.as_dvec3(),
			age: 0.,
			phase: p.phase as f64,
			birth: p.birth as f64,
			affine: Mat3::IDENTITY,
			previous_model: Mat4::IDENTITY,
			pose: p,
			previous: p,
			concentration: 1.,
		};
		b.previous_model = b.model();
		b
	}
	fn shape(b: &mut BodyMotion, p: Pose) {
		b.scale = p.scale.as_dvec3();
		b.phase = p.phase as f64;
		b.pose = p;
		b.pose.rotation = b.rotation.as_quat();
		b.pose.pivot = Vec3::ZERO;
		b.pose.position = b.position.as_vec3()
			- b.pose.rotation * (p.coefficients.map(Vec3::new(0.05, 0.66, 0.)) * p.scale);
	}
	#[cfg(test)]
	fn advance(&mut self, target: f64, parent: Mat4, scene: &InteractionScene) {
		self.advance_profiled(target, parent, scene, None);
	}
	fn advance_profiled(
		&mut self,
		target: f64,
		parent: Mat4,
		scene: &InteractionScene,
		mut profile: Option<&mut Probe>,
	) {
		let end = (target / DT + 1e-6).floor() as u32;
		if end < self.tick {
			self.tick = 0;
			self.bodies.fill(None);
			self.history.clear();
			self.contact_sides.fill(0.);
			let initial = self.region.as_ref().map(|r| r.scene(0.));
			self.field = self.contact.then(|| Flow::new(initial.as_ref().unwrap_or(scene).wind));
			self.surface_ticks.fill(u32::MAX);
		}
		for _ in 0..32 {
			if self.tick >= end {
				break;
			}
			let t = self.tick as f64 * DT;
			let local_scene = self.region.as_ref().map(|r| r.scene(t));
			let scene = local_scene.as_ref().unwrap_or(scene);
			if self.tick % 2 == 0 {
				if let Some(f) = &mut self.field {
					timed!(profile, Flow, f.step(scene, t));
					if let Some(p) = profile.as_deref_mut() {
						p.frame.flow_steps += 1;
					}
				}
			}
			BodyMotion::remember(&mut self.history, scene, t);
			let mut cached = [false; 16];
			for id in 0..self.count() {
				let Some((pose, _, _)) = timed!(profile, Authored, self.authored(t + DT, id))
				else {
					self.bodies[id] = None;
					continue;
				};
				let p = Self::world_pose(pose, parent);
				if self.bodies[id].is_none() {
					let previous =
						self.authored(t, id).map(|(p, _, _)| Self::world_pose(p, parent));
					let v = previous.map_or_else(
						|| {
							self.authored(t + 2. * DT, id)
								.map(|(q, _, _)| {
									let q = Self::world_pose(q, parent);
									(q.world(Vec3::new(0.05, 0.66, 0.))
										- p.world(Vec3::new(0.05, 0.66, 0.)))
									.as_dvec3() / DT
								})
								.unwrap_or(DVec3::ZERO)
						},
						|old| {
							(p.world(Vec3::new(0.05, 0.66, 0.))
								- old.world(Vec3::new(0.05, 0.66, 0.)))
							.as_dvec3() / DT
						},
					);
					self.bodies[id] = Some(Self::initialize(
						id,
						p,
						v.clamp_length_max(if self.landing { 3. } else { 1.2 }),
					));
				}
				let Some(b) = &mut self.bodies[id] else {
					continue;
				};
				b.previous = b.pose;
				b.previous_model = b.model();
				Self::shape(b, p);
				if let Some(f) = &self.field {
					timed!(profile, Advect, b.advect(f, scene, &self.history, t));
					if self.part.refined {
						timed!(
							profile,
							Response,
							crate::parts::cartoon_flow::contact::respond(
								b,
								&mut self.contact_sides[id],
								scene,
								t,
							)
						);
					}
					b.turn();
					b.age = t + DT - b.birth;
				}
				Self::shape(b, p);
				if let Some(rest) = &self.rest {
					let model = b.model();
					let floor = timed!(
						profile,
						Ground,
						if self.part.refined {
							let source = id / 3;
							if !cached[source] {
								self.ground_cache[source].clear();
								self.ground_cache[source].extend(rest[p.variant].iter().map(|r| {
									if p.coefficients.0 == Vec4::ZERO {
										*r
									} else {
										p.coefficients.map(*r)
									}
								}));
								cached[source] = true;
							}
							self.ground_cache[source]
								.iter()
								.map(|r| model.transform_point3(*r).y)
								.fold(f32::INFINITY, f32::min)
						} else {
							rest[p.variant]
								.iter()
								.map(|r| model.transform_point3(p.coefficients.map(*r)).y)
								.fold(f32::INFINITY, f32::min)
						}
					);
					let ground = if self.part.refined { 0.012 } else { 0.004 };
					if floor < ground {
						b.position.y += (ground - floor) as f64;
						b.velocity.y = b.velocity.y.max(0.);
						Self::shape(b, p);
					}
				}
			}
			self.tick += 1;
			if let Some(p) = profile.as_deref_mut() {
				p.frame.ticks += 1;
			}
		}
	}
}
fn tick(
	mut commands: Commands,
	mut optical: Query<&mut crate::parts::combined_choreography::OpticalChoreography>,
	optical_assets: (
		Option<Res<Assets<crate::parts::combined::assets::CombinedAsset>>>,
		Option<Res<Assets<crate::parts::packaged::StaticVolume>>>,
		Option<Res<Assets<Image>>>,
		Query<(&Camera, Has<bevy::core_pipeline::prepass::DepthPrepass>), With<Camera3d>>,
		Res<region::ContactRegionBudget>,
	),
	mut probe: Option<ResMut<Probe>>,
	server: Res<AssetServer>,
	mut meshes: ResMut<Assets<Mesh>>,

	mut closed: ResMut<surface::ClosedFields>,
	mut geometry: ResMut<Geometry>,
	mut materials: ResMut<Assets<LandingMaterial>>,
	clock: Res<Time<Virtual>>,
	scene: Res<InteractionScene>,
	instances: Query<
		(
			&VfxInstance,
			Option<&region::ContactInputs>,
			Option<&crate::spawn::VfxPendingLayers>,
			&Transform,
		),
		Without<Surface>,
	>,
	combined: Query<&crate::parts::combined::CombinedState>,
	mut layers: Query<(
		Entity,
		&VfxMemberOf,
		Ref<GlobalTransform>,
		&mut CartoonFlow,
		&mut VfxLayerLife,
	)>,
	mut surfaces: Query<(
		Entity,
		&mut Surface,
		&Mesh3d,
		&mut Transform,
		&mut Visibility,
		&mut Aabb,
	)>,
) {
	let (assets, volumes, images, cameras, budget) = optical_assets;
	let mut profile = probe.as_deref_mut();
	let tick_stamp = profile.as_ref().map(|_| std::time::Instant::now());
	let mut optical_admitted = optical.iter().filter(|o| o.ready && !o.contact()).count();
	let mut owner = layers
		.iter()
		.filter(|(_, _, _, s, _)| s.ready && s.contact && s.region.is_none())
		.map(|(e, _, _, _, _)| e)
		.min();
	let mut admitted =
		layers.iter().filter(|(_, _, _, s, _)| s.ready && s.region.is_some()).count();
	let mut ids = layers.iter().map(|(e, _, _, _, _)| e).collect::<Vec<_>>();
	ids.sort();
	for e in ids {
		let Ok((entity, member, global, mut s, mut life)) = layers.get_mut(e) else {
			continue;
		};
		let Ok((instance, inputs, pending, root_transform)) = instances.get(member.root()) else {
			continue;
		};
		let mut optical = optical.get_mut(entity).ok();
		if !s.ready {
			if let Some(o) = optical.as_deref_mut() {
				// The host/layer must pass through transform propagation before validation.
				if global.is_added() {
					continue;
				}
				if optical_admitted >= 4 && !s.part.refined {
					warn!("Combined choreography rejected: four active events");
					commands.entity(member.root()).try_despawn();
					continue;
				}
				if cameras.iter().any(|(c, d)| c.is_active && !d) {
					warn!("Combined choreography requires DepthPrepass on active cameras");
					commands.entity(member.root()).try_despawn();
					continue;
				}
				if !o.valid_placement(global.to_matrix()) {
					warn!("Combined choreography placement rejected");
					commands.entity(member.root()).try_despawn();
					continue;
				}
				if server.get_load_states(o.part.assets.id()).is_some_and(|(s, _, d)| {
					matches!(s, bevy::asset::LoadState::Failed(_))
						|| matches!(d, bevy::asset::RecursiveDependencyLoadState::Failed(_))
				}) {
					commands.entity(member.root()).try_despawn();
					continue;
				}
				let (Some(a), Some(v), Some(i)) =
					(assets.as_deref(), volumes.as_deref(), images.as_deref())
				else {
					continue;
				};
				if a.get(&o.part.assets).is_some_and(|a| a.meshes != o.part.meshes) {
					warn!("Combined choreography mesh/rest-field mapping mismatch");
					commands.entity(member.root()).try_despawn();
					continue;
				}
				if !o.dependencies(a, v, i) {
					continue;
				}
				life.duration = o.duration();
			}
			if s.contact
				&& !s.part.refined
				&& (owner.is_some() || combined.iter().any(|c| c.options.motion.interactive()))
			{
				warn!("Cartoon interaction rejected: interactive domain already owned");
				commands.entity(member.root()).try_despawn();
				continue;
			}
			if s.contact && s.part.refined {
				if global.is_added() {
					continue;
				}
				let frame = region::ContactFrame::new(global.to_matrix());
				if admitted >= budget.maximum
					|| frame.is_err()
					|| pending
						.is_some_and(|p| !p.spawn.transform.scale.abs_diff_eq(Vec3::ONE, 1e-5))
				{
					warn!(
						"Contact region rejected: capacity {} or unsupported placement",
						budget.maximum
					);
					commands.entity(member.root()).try_despawn();
					continue;
				}
				let Ok(frame) = frame else {
					continue;
				};
				let mut base =
					if inputs.is_some() { InteractionScene::default() } else { scene.clone() };
				base.compact_landing = s.landing;
				base.cartoon_wave_timing = !s.landing;
				s.region = Some(region::ContactRegion {
					frame,
					root_transform: root_transform.to_matrix(),
					inputs: inputs.cloned(),
					base,
				});
			}
			let Some(rest) = geometry.load(&s.part.meshes, &meshes) else {
				if s.part.meshes.iter().any(|h| {
					server
						.get_load_states(h.id())
						.is_some_and(|(v, _, _)| matches!(v, bevy::asset::LoadState::Failed(_)))
				}) {
					commands.entity(member.root()).try_despawn();
				}
				continue;
			};
			if s.part.refined
				&& !timed!(profile, Cache, closed.prepare(&mut meshes, &s.part.meshes))
			{
				continue;
			}
			s.wave = Some(Wave::new(rest.clone()));
			s.rest = Some(if s.part.refined {
				std::array::from_fn(|i| closed.fields[i].support.clone())
			} else {
				rest
			});
			s.specs = motion::Puff::table(instance.seed as u32);
			if s.part.refined && s.landing && optical.is_none() {
				for i in 0..16 {
					let spec = &s.specs[i];
					s.surface_noise[i] = timed!(
						profile,
						NoiseInit,
						closed.fields[spec.variant].noise_grid(spec.seed)
					);
				}
			}
			let landing = s.landing;
			let initial = s.region.as_ref().map(|r| r.scene(0.));
			s.field = s.contact.then(|| Flow::new(initial.as_ref().unwrap_or(&scene).wind));
			if s.contact && s.region.is_none() {
				commands.queue(move |w: &mut World| {
					let mut scene = w.resource_mut::<InteractionScene>();
					scene.compact_landing = landing;
					scene.cartoon_wave_timing = !landing;
					scene.changed();
				});
			}
			s.ready = true;
			if let Some(o) = optical.as_deref_mut() {
				o.ready = true;
				optical_admitted += 1;
			}
			if s.region.is_some() {
				admitted += 1;
			}
			if s.contact && s.region.is_none() {
				owner = Some(e);
			}
			life.waiting_for_emitter = false;
			for index in 0..if optical.is_none() { s.count() } else { 0 } {
				let source = if s.part.refined { index / 3 } else { index };
				let variant = if s.landing {
					s.specs[source].variant
				} else if source < 12 {
					source % 3
				} else {
					0
				};
				let material = materials.add(if s.landing {
					LandingMaterial {
						cutoff: Vec4::splat(-0.02),
						seed: UVec4::new(s.specs[source].seed, 0, 0, 0),
						environment: default(),
					}
				} else {
					LandingMaterial::wave(Vec4::ZERO)
				});
				let dynamic = if s.part.refined { Some(meshes.reserve_handle()) } else { None };
				commands.spawn((
					ChildOf(entity),
					VfxMemberOf(member.root()),
					Mesh3d(if s.part.refined {
						closed.fields[variant].base.clone()
					} else {
						s.part.meshes[variant].clone()
					}),
					MeshMaterial3d(material.clone()),
					Transform::IDENTITY,
					Visibility::Hidden,
					Aabb::from_min_max(Vec3::splat(-4.), Vec3::splat(4.)),
					Surface {
						index,
						controller: entity,
						material,
						dynamic,
						geometry_tick: u32::MAX,
						geometry_scene: u64::MAX,
						geometry_empty: false,
					},
				));
			}
		}
		if let Some(r) = &mut s.region {
			if !r.frame.world_from_local.abs_diff_eq(global.to_matrix(), 1e-5)
				|| !r.root_transform.abs_diff_eq(root_transform.to_matrix(), 1e-5)
			{
				warn!("Contact region root moved after admission; cancelling {:?}", member.root());
				commands.entity(member.root()).try_despawn();
				continue;
			}
			if let Some(input) = inputs {
				r.inputs = Some(input.clone());
			} else if r.inputs.is_none() {
				let landing = r.base.compact_landing;
				r.base = scene.clone();
				r.base.compact_landing = landing;
				r.base.cartoon_wave_timing = !landing;
			}
		}
		let parent = if s.region.is_some() { Mat4::IDENTITY } else { global.to_matrix() };
		if instance.armed && !clock.is_paused() {
			if let Some(o) = optical.as_deref_mut() {
				if let Some(a) = assets.as_deref().and_then(|a| a.get(&o.part.assets)) {
					s.advance_optical(instance.age.min(life.duration) as f64, parent, &scene, o, a);
				}
			} else {
				s.advance_profiled(
					instance.age.min(life.duration) as f64,
					parent,
					&scene,
					profile.as_deref_mut(),
				);
			}
		}
		if let Some(p) = profile.as_deref_mut() {
			p.frame.age = instance.age;
			p.frame.tick = s.tick;
			p.frame.debt = ((instance.age.min(life.duration) as f64 / DT + 1e-6).floor() as u32)
				.saturating_sub(s.tick);
			p.frame.bodies = s.bodies.iter().flatten().count();
		}
		// Retire only after fixed-step catch-up reaches the authored ending.
		// A slow frame must not discard unprocessed simulation time.
		life.age = s.tick as f32 / 120.;
	}
	for (surface_entity, mut body, mesh, mut transform, mut visible, mut bounds) in &mut surfaces {
		let Ok((_, member, global, mut s, _)) = layers.get_mut(body.controller) else {
			continue;
		};
		let armed = instances.get(member.root()).is_ok_and(|(i, _, _, _)| i.armed);
		let Some(b) = s.bodies[body.index].clone().filter(|_| armed) else {
			*visible = Visibility::Hidden;
			continue;
		};
		let t = s.tick as f64 * DT;
		let local_scene = s.region.as_ref().map(|r| r.scene(t));
		let scene = local_scene.as_ref().unwrap_or(&scene);
		let Some((p, threshold, seed)) = timed!(profile, Authored, s.authored(t, body.index))
		else {
			*visible = Visibility::Hidden;
			continue;
		};
		*transform = p.transform();
		*visible = Visibility::Visible;
		if s.part.refined && (body.geometry_tick != s.tick || body.geometry_scene != scene.version)
		{
			let field = &closed.fields[p.variant];
			let near = timed!(profile, Broad, field.needs_contact(&b, &scene, t));
			if let Some(p) = profile.as_deref_mut() {
				p.frame.broad_checks += 1;
				p.affected[body.index] = near || (s.landing && threshold >= 0.);
				p.near_solid[body.index] = near;
			}
			if (near || (s.landing && threshold >= 0.))
				&& profile.as_ref().is_none_or(|p| p.mode != Mode::NoRebuild)
			{
				let source = body.index / 3;
				if s.surface_ticks[source] != s.tick {
					s.surface_nodes[source].clear();
					timed!(
						profile,
						Deform,
						s.surface_nodes[source].extend(field.nodes.iter().map(|r| {
							if p.coefficients.0 == Vec4::ZERO {
								*r
							} else {
								p.coefficients.map(*r)
							}
						}))
					);
					s.surface_ticks[source] = s.tick;
				}
				if let Some(handle) = body.dynamic.as_ref() {
					let generated = field.rebuild_profiled(
						&b,
						&scene,
						t,
						if s.landing { threshold } else { -1. },
						if s.landing { Some(&s.surface_noise[source]) } else { None },
						&s.surface_nodes[source],
						profile.as_deref_mut(),
					);
					if let Some(p) = profile.as_deref_mut() {
						p.frame.rebuilds += 1;
						p.frame.vertices += generated.count_vertices();
						p.frame.triangles +=
							generated.indices().map_or(generated.count_vertices(), |v| v.len()) / 3;
					}
					let handle = handle.clone();
					body.geometry_empty = generated.count_vertices() == 0;
					if !body.geometry_empty
						&& profile.as_ref().is_none_or(|p| p.mode != Mode::NoUpload)
					{
						if let Some(p) = profile.as_deref_mut() {
							p.frame.replacements += 1;
							p.frame.upload_bytes += generated.count_vertices() * 24
								+ generated.indices().map_or(0, |v| v.len() * 4);
						}
						if let Err(e) =
							timed!(profile, Insert, meshes.insert(handle.id(), generated))
						{
							warn!("Contact geometry asset: {e}");
							commands.entity(member.root()).try_despawn();
							continue;
						}
						if mesh.0 != handle {
							commands.entity(surface_entity).try_insert(Mesh3d(handle));
						}
					}
				}
			} else {
				if mesh.0 != field.base {
					commands.entity(surface_entity).try_insert(Mesh3d(field.base.clone()));
				}
				body.geometry_empty = false;
			}
			body.geometry_tick = s.tick;
			body.geometry_scene = scene.version;
		}
		if s.part.refined && body.geometry_empty {
			*visible = Visibility::Hidden;
		}
		let binding_stamp = profile.as_ref().map(|_| std::time::Instant::now());
		if let Some(p) = profile.as_deref_mut() {
			p.frame.affected += usize::from(p.affected[body.index]);
			p.frame.near_solid += usize::from(p.near_solid[body.index]);
		}
		let original = global.to_matrix() * p.transform().to_matrix();
		let placement = s.region.as_ref().map_or(Mat4::IDENTITY, |r| r.frame.world_from_local);
		let map = placement * b.model() * original.inverse();
		if let Some(current) = materials.get(&body.material) {
			let mut m = current.clone();
			if s.landing && !s.part.refined {
				m.cutoff.x = threshold;
				m.seed.x = seed;
			} else {
				m.set_wave_coefficients(p.coefficients.0);
			}
			m.environment = SurfaceEnvironment::new(map, &scene, t);
			if s.part.refined {
				m.seed.w = 1;
				m.seed.z = 2;
				m.environment.settings.x = 0.;
			}

			if m.cutoff != current.cutoff
				|| m.seed != current.seed
				|| m.environment != current.environment
			{
				if let Some(mut target) = materials.get_mut(&body.material) {
					*target = m;
				}
			}
		}
		let correction = original.inverse() * map * original;
		let (lo, hi) = crate::parts::combined::render::Primitive::transformed_bounds(
			correction,
			Vec3::new(-2.042, -0.444, -1.817),
			Vec3::new(1.917, 3.531, 1.786),
		);
		*bounds = Aabb::from_min_max(lo, hi);
		if let (Some(p), Some(t)) = (profile.as_deref_mut(), binding_stamp) {
			p.frame.ns[diagnostics::Stage::Bindings as usize] += t.elapsed().as_nanos() as u64;
		}
	}
	if let (Some(p), Some(t)) = (profile, tick_stamp) {
		p.frame.ns[diagnostics::Stage::Tick as usize] += t.elapsed().as_nanos() as u64;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		composition::{EffectDefinition, EffectLayer, EffectPart},
		lifecycle::{gate_vfx_instances, tick_vfx_instances},
		spawn::spawn_vfx,
	};
	use bevy::{asset::AssetPlugin, time::TimeUpdateStrategy};
	use std::time::Duration;
	fn controller(landing: bool) -> CartoonFlow {
		let rest: [Arc<[Vec3]>; 3] = std::array::from_fn(|_| {
			Arc::from([Vec3::new(-0.6, 0., -0.5), Vec3::new(0.6, 1.3, 0.5)])
		});
		CartoonFlow {
			landing,
			contact: true,
			region: None,
			tick: 0,
			part: CartoonFlowPart { meshes: default(), landing, refined: false },
			wave: Some(Wave::new(rest.clone())),
			rest: Some(rest),
			specs: motion::Puff::table(13),
			bodies: vec![None; 16],
			field: Some(Flow::new(InteractionScene::default().wind)),
			history: VecDeque::new(),
			ready: true,
			surface_nodes: std::array::from_fn(|_| Vec::new()),
			surface_ticks: [u32::MAX; 16],
			surface_noise: std::array::from_fn(|_| Vec::new()),
			contact_sides: [0.; 48],
			ground_cache: std::array::from_fn(|_| Vec::new()),
		}
	}
	#[test]
	fn gpu_map_normals_and_bounds_keep_shear_and_parent_pose() -> Result<(), String> {
		let c = controller(false);
		let (p, _, _) = c.authored(0.9, 2).ok_or("pose")?;
		let parent = Mat4::from_scale_rotation_translation(
			Vec3::splat(1.7),
			Quat::from_rotation_y(0.6),
			Vec3::new(3., 0., -2.),
		);
		let original = parent * p.transform().to_matrix();
		let mut b = CartoonFlow::initialize(2, CartoonFlow::world_pose(p, parent), DVec3::ZERO);
		b.affine = Mat3::from_cols(
			Vec3::new(1., 0.08, 0.),
			Vec3::new(0.15, 1., -0.07),
			Vec3::new(0.02, 0., 1.),
		);
		b.position += DVec3::new(0.3, 0.1, -0.2);
		CartoonFlow::shape(&mut b, CartoonFlow::world_pose(p, parent));
		let env = SurfaceEnvironment::new(
			b.model() * original.inverse(),
			&InteractionScene::default(),
			0.,
		);
		let correction = original.inverse() * env.map * original;
		let lo = Vec3::new(-2.042, -0.444, -1.817);
		let hi = Vec3::new(1.917, 3.531, 1.786);
		let (a, z) =
			crate::parts::combined::render::Primitive::transformed_bounds(correction, lo, hi);
		for rest in [Vec3::ZERO, Vec3::new(0.2, 0.8, -0.3), Vec3::new(-0.5, 1.1, 0.4)] {
			let local = p.coefficients.map(rest);
			let gpu = env.map.transform_point3(original.transform_point3(local));
			let (cpu, _) = b.material_point(rest);
			assert!(gpu.distance(cpu.as_vec3()) < 2e-5);
			let q = correction.transform_point3(local);
			assert!(q.cmpge(a - Vec3::splat(1e-5)).all() && q.cmple(z + Vec3::splat(1e-5)).all());
			let n = p.coefficients.normal(rest, Vec3::Y);
			let n0 = original.inverse().transpose().transform_vector3(n).normalize();
			let gpu_n = env.normal.transform_vector3(n0).normalize();
			let expected = b.model().inverse().transpose().transform_vector3(n).normalize();
			assert!(gpu_n.distance(expected) < 2e-5);
		}
		Ok(())
	}
	#[test]
	fn fixed_partition_bounded_affine_and_no_authored_snapback() -> Result<(), String> {
		for landing in [false, true] {
			let mut a = controller(landing);
			let mut b = controller(landing);
			let scene = InteractionScene {
				compact_landing: landing,
				cartoon_wave_timing: !landing,
				..default()
			};
			let end = if landing { 0.7 } else { 3.4 };
			while a.tick as f64 * DT + 1e-8 < end {
				a.advance(end, Mat4::IDENTITY, &scene);
			}
			let mut t = 0.;
			let mut probe = Probe::default();
			while t < end {
				t = (t + 0.037f64).min(end);
				b.advance_profiled(t, Mat4::IDENTITY, &scene, Some(&mut probe));
			}
			assert_eq!(a.tick, b.tick);
			assert_eq!(probe.frame.ticks, b.tick);
			assert_eq!(probe.frame.flow_steps, (b.tick + 1) / 2);
			for (x, y) in a.bodies.iter().zip(&b.bodies) {
				if let (Some(x), Some(y)) = (x, y) {
					assert_eq!(x.position, y.position);
					assert_eq!(x.affine, y.affine);
					assert!(x.affine.determinant() > 0.);
					let norm = (x.affine - Mat3::IDENTITY)
						.to_cols_array()
						.iter()
						.map(|v| v * v)
						.sum::<f32>()
						.sqrt();
					assert!(norm <= scene.flow_limit + 1e-6);
				}
			}
			let id = a.bodies.iter().position(Option::is_some).ok_or("body")?;
			let body = a.bodies[id].as_mut().ok_or("body")?;
			body.position.z += 0.8;
			let z = body.position.z;
			a.advance(end + DT, Mat4::IDENTITY, &scene);
			assert!(
				(a.bodies[id].as_ref().ok_or("body after release")?.position.z - z).abs() < 0.1
			);
			if landing {
				while a.tick < 144 {
					a.advance(1.2, Mat4::IDENTITY, &scene);
				}
				assert!(a.bodies.iter().all(Option::is_none));
			}
		}
		Ok(())
	}
	#[test]
	fn landing_proxy_timing_and_carry_control() -> Result<(), String> {
		use crate::parts::combined::interaction::Scenario;
		let mut scene = InteractionScene { compact_landing: true, ..default() };
		assert!((scene.capsule(0.2).linear - DVec3::Z * 2.).length() < 1e-12);
		scene.scenario = Scenario::Fast;
		assert!((scene.capsule(0.2).linear - DVec3::Z * 8.).length() < 1e-12);
		scene.scenario = Scenario::Stop;
		assert_eq!(scene.capsule(0.5).linear, DVec3::ZERO);
		for landing in [false, true] {
			let end = if landing { 0.7 } else { 3.8 };
			let mut displacement = Vec::new();
			for scenario in
				[Scenario::Cross, Scenario::Stop, Scenario::Fast, Scenario::Static, Scenario::None]
			{
				let mut states = Vec::new();
				for carry in [1., 0.15] {
					let mut c = controller(landing);
					let scene = InteractionScene {
						compact_landing: landing,
						cartoon_wave_timing: !landing,
						scenario,
						clump_carry: carry,
						..default()
					};
					while c.tick as f64 * DT + 1e-8 < end {
						c.advance(end, Mat4::IDENTITY, &scene);
					}
					let points = c
						.bodies
						.iter()
						.flatten()
						.map(|b| {
							assert!(b.position.is_finite() && b.velocity.is_finite());
							assert!(b.velocity.length() < 10.);
							assert!(b.affine.determinant() > 0.);
							b.position
						})
						.collect::<Vec<_>>();
					states.push(points);
				}
				if matches!(scenario, Scenario::None | Scenario::Static) {
					assert_eq!(states[0], states[1]);
				} else {
					let sum = states[0].iter().zip(&states[1]).map(|(a, b)| a.z - b.z).sum::<f64>();
					println!(
						"Cartoon landing={landing} scenario={scenario:?} sum baseline-minus-tuned Z={sum}"
					);
					displacement.push(sum);
				}
			}
			assert!(displacement.iter().any(|v| *v > 0.001));
		}
		Ok(())
	}

	#[test]
	fn host_admission_pause_speed_cancel_and_cleanup() -> Result<(), Box<dyn std::error::Error>> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<LandingMaterial>()
			.init_asset::<Image>()
			.init_asset::<crate::parts::packaged::StaticVolume>()
			.init_resource::<surface::ClosedFields>()
			.init_resource::<region::ContactRegionBudget>()
			.init_resource::<Geometry>()
			.init_resource::<InteractionScene>()
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(10)))
			.add_systems(Update, (tick_vfx_instances, tick).chain())
			.add_systems(PostUpdate, gate_vfx_instances);
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Cuboid::default());
		let def = EffectDefinition::new(
			"cartoon-landing-interactive",
			[EffectLayer {
				part: EffectPart::CartoonFlow(CartoonFlowPart {
					refined: false,
					meshes: [mesh.clone(), mesh.clone(), mesh],
					landing: true,
				}),
				delay: 0.,
				transform: Transform::IDENTITY,
				scale: 1.,
			}],
		);
		let spawn = |app: &mut App| {
			let root = spawn_vfx(
				&mut app.world_mut().commands(),
				&def,
				VfxSpawn { seed: Some(13), playback: 2., ..default() },
			);
			app.world_mut().flush();
			root
		};
		let a = spawn(&mut app);
		for _ in 0..8 {
			app.update();
		}
		let age = app.world().get::<VfxInstance>(a).ok_or("owner")?.age;
		assert!(age > 0.08);
		let n = app.world_mut().query::<&Surface>().iter(app.world()).count();
		assert_eq!(n, 16);
		assert_eq!(
			app.world_mut()
				.query::<&crate::parts::combined::CombinedState>()
				.iter(app.world())
				.count(),
			0
		);
		let tick = app.world_mut().query::<&CartoonFlow>().single(app.world())?.tick;
		app.world_mut().resource_mut::<Time<Virtual>>().pause();
		for _ in 0..4 {
			app.update();
		}
		assert_eq!(app.world_mut().query::<&CartoonFlow>().single(app.world())?.tick, tick);
		let excess = spawn(&mut app);
		app.update();
		assert!(app.world().get::<VfxInstance>(excess).is_none());
		assert!(app.world().get::<VfxInstance>(a).is_some());
		app.world_mut().despawn(a);
		app.update();
		assert_eq!(app.world_mut().query::<&Surface>().iter(app.world()).count(), 0);
		assert_eq!(app.world_mut().query::<&CartoonFlow>().iter(app.world()).count(), 0);
		app.world_mut().resource_mut::<Time<Virtual>>().unpause();
		let replay = spawn(&mut app);
		for _ in 0..80 {
			app.update();
		}
		assert!(app.world().get::<VfxInstance>(replay).is_none());
		for _ in 0..5 {
			app.update();
		}
		assert_eq!(app.world().resource::<Assets<LandingMaterial>>().len(), 0);
		let closed =
			surface::ClosedFields::fixture(&mut app.world_mut().resource_mut::<Assets<Mesh>>());
		app.insert_resource(closed);
		let shared = app.world().resource::<Assets<Mesh>>().len();
		let mut refined = def.clone();
		if let EffectPart::CartoonFlow(p) = &mut refined.layers[0].part {
			p.refined = true;
		}
		for _ in 0..2 {
			let root = spawn_vfx(
				&mut app.world_mut().commands(),
				&refined,
				VfxSpawn { seed: Some(13), ..default() },
			);
			app.world_mut().flush();
			for _ in 0..6 {
				app.update();
			}
			assert_eq!(app.world_mut().query::<&Surface>().iter(app.world()).count(), 48);
			assert!(app.world().resource::<Assets<Mesh>>().len() <= shared + 48);
			app.world_mut().despawn(root);
			for _ in 0..6 {
				app.update();
			}
			assert_eq!(app.world_mut().query::<&Surface>().iter(app.world()).count(), 0);
			assert_eq!(app.world().resource::<Assets<LandingMaterial>>().len(), 0);
			assert_eq!(app.world().resource::<Assets<Mesh>>().len(), shared);
		}
		Ok(())
	}
	#[test]
	fn refined_contact_is_partition_independent_in_all_scenarios() -> Result<(), String> {
		use crate::parts::combined::interaction::Scenario;
		for landing in [false, true] {
			for scenario in [
				Scenario::Cross,
				Scenario::Stop,
				Scenario::Fast,
				Scenario::Grazing,
				Scenario::Static,
				Scenario::None,
			] {
				let scene = InteractionScene {
					compact_landing: landing,
					cartoon_wave_timing: !landing,
					scenario,
					..default()
				};
				let end = if landing { 0.8 } else { 3.5 };
				let mut states = Vec::new();
				for partition in [0.037f64, 0.119] {
					let mut c = controller(landing);
					c.part.refined = true;
					c.bodies = vec![None; 48];
					let mut time = 0.;
					while time < end {
						time = (time + partition).min(end);
						c.advance(time, Mat4::IDENTITY, &scene);
						for b in c.bodies.iter().flatten() {
							assert!(b.position.is_finite() && b.velocity.is_finite());
							assert!(b.velocity.length() < 10. && b.affine.determinant() > 0.);
						}
					}
					states.push(
						c.bodies
							.iter()
							.map(|b| {
								b.as_ref().map(|b| (b.position, b.velocity, b.affine, b.rotation))
							})
							.collect::<Vec<_>>(),
					);
				}
				assert_eq!(states[0], states[1], "{landing} {scenario:?}");
			}
		}
		Ok(())
	}
	#[test]
	fn refined_lobes_are_prepared_bounded_and_owned() -> Result<(), String> {
		for landing in [false, true] {
			let mut c = controller(landing);
			c.part.refined = true;
			c.bodies = vec![None; 48];
			assert_eq!(c.count(), if landing { 48 } else { 45 });
			let scene = InteractionScene {
				compact_landing: landing,
				cartoon_wave_timing: !landing,
				..default()
			};
			let end = if landing { 1.2 } else { 4. };
			while c.tick as f64 * DT + 1e-8 < end {
				c.advance((c.tick as f64 * DT + 0.05).min(end), Mat4::IDENTITY, &scene);
				for b in c.bodies.iter().flatten() {
					assert!(b.model().is_finite());
					assert!(b.affine.determinant() > 0.);
					assert!(b.velocity.length() < 10.);
				}
			}
			if landing {
				assert!(c.bodies.iter().all(Option::is_none));
			}
		}
		Ok(())
	}
	fn optical_fixture(
		landing: bool,
		contact: bool,
	) -> Result<
		(
			CartoonFlow,
			crate::parts::combined_choreography::OpticalChoreography,
			crate::parts::combined::assets::CombinedAsset,
		),
		Box<dyn std::error::Error>,
	> {
		use crate::{
			composition::*,
			parts::{combined::assets::*, combined_choreography::OpticalChoreography},
		};
		let mut c = controller(landing);
		c.contact = contact;
		c.part.refined = contact;
		c.bodies = vec![None; if contact { 48 } else { 16 }];
		if !contact {
			c.field = None;
		}
		let manifest: Manifest = serde_json::from_str(include_str!(
			"../../../../assets/vfx/dust/combined.combined.json"
		))?;
		let config: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../assets/vfx/dust/clouds/dynamic_rules.json"
		))?;
		let style: serde_json::Value = serde_json::from_str(include_str!(
			"../../../../assets/vfx/dust/bodies/body_assets.json"
		))?;
		let asset = CombinedAsset {
			manifest,
			config: serde_json::from_value(config["config"].clone())?,
			style: serde_json::from_value(style["combined_shading"].clone())?,
			bodies: default(),
			clouds: default(),
			meshes: default(),
		};
		let o = OpticalChoreography {
			part: CombinedChoreographyPart {
				assets: default(),
				meshes: default(),
				landing,
				contact,
				options: default(),
			},
			particles: Vec::new(),
			seed: 13,
			dropped: [0; 2],
			ready: true,
			body_ns: 0,
			particle_ns: 0,
			steps: 0,
		};
		Ok((c, o, asset))
	}
	#[test]
	fn contact_rigid_equivariance_body_flow_particles_and_retirement(
	) -> Result<(), Box<dyn std::error::Error>> {
		use region::*;
		for landing in [false, true] {
			let (mut a, mut ao, asset) = optical_fixture(landing, true)?;
			let (mut b, mut bo, _) = optical_fixture(landing, true)?;
			let base = InteractionScene {
				compact_landing: landing,
				cartoon_wave_timing: !landing,
				..default()
			};
			let f = ContactFrame::new(Mat4::from_rotation_translation(
				Quat::from_rotation_y(0.64577),
				Vec3::new(-12., 8., 18.),
			))?;
			let identity = ContactFrame::new(Mat4::IDENTITY)?;
			let mut ai = ContactInputs::default();
			let mut bi = ContactInputs::default();
			for tick in (0..=684).step_by(4) {
				let t = tick as f64 * DT;
				let colliders =
					base.colliders(t).into_iter().take(base.collider_count()).collect::<Vec<_>>();
				ai.submit(WorldContactSnapshot {
					age: t,
					colliders: colliders.clone(),
					wind: base.wind,
					discontinuous: tick == 0,
				})?;
				bi.submit(WorldContactSnapshot {
					age: t,
					colliders: colliders.into_iter().map(|c| f.world_collider(c)).collect(),
					wind: f.world_vector(base.wind),
					discontinuous: tick == 0,
				})?;
			}
			a.region = Some(ContactRegion {
				frame: identity,
				root_transform: Mat4::IDENTITY,
				inputs: Some(ai),
				base: base.clone(),
			});
			b.region = Some(ContactRegion {
				frame: f,
				root_transform: f.world_from_local,
				inputs: Some(bi),
				base: base.clone(),
			});
			for target in [0.2_f64, 0.5, 0.9, 1.1, 2., 3.5] {
				let target: f64 = target.min(if landing { 1.1 } else { 3.5 });
				while (a.tick as f64) * DT + 1e-8 < target {
					a.advance_optical(target, Mat4::IDENTITY, &base, &mut ao, &asset);
					b.advance_optical(target, Mat4::IDENTITY, &base, &mut bo, &asset);
				}
				assert_eq!(a.tick, b.tick);
				assert_eq!(ao.particles.len(), bo.particles.len());
				for (a, b) in a.bodies.iter().zip(&b.bodies) {
					if let (Some(a), Some(b)) = (a, b) {
						assert!((a.position - b.position).length() < 1e-5);
						assert!((a.velocity - b.velocity).length() < 1e-5);
						assert!(a.affine.abs_diff_eq(b.affine, 1e-5));
						assert!(a.model().abs_diff_eq(b.model(), 1e-5));
						assert!(b.position.y >= 0.);
					} else {
						assert_eq!(a.is_some(), b.is_some());
					}
				}
				for (a, b) in ao.particles.iter().zip(&bo.particles) {
					assert!((a.position - b.position).length() < 2e-5);
					assert!((a.velocity - b.velocity).length() < 2e-5);
					assert!(b.position.y >= 0.);
				}
				for p in [DVec3::new(0.2, 0.7, 0.3), DVec3::new(1., 1., -1.)] {
					assert!(
						(a.field.as_ref().ok_or("field")?.sample(p)
							- b.field.as_ref().ok_or("field")?.sample(p))
						.length() < 1e-5
					);
				}
				for i in 0..a.count() {
					assert_eq!(a.retirement(i), b.retirement(i));
				}
				let supports =
					std::array::from_fn(|_| (Vec3::new(-1., 0., -1.), Vec3::new(1., 1.4, 1.)));
				for (p, q) in ao.primitives(&a, supports).iter().zip(bo.primitives(&b, supports)) {
					assert!(p.inverse.abs_diff_eq(q.inverse, 2e-4));
				}
			}
		}
		Ok(())
	}

	#[test]
	fn optical_landing_reference_retirement_complete_inverse_and_tail(
	) -> Result<(), Box<dyn std::error::Error>> {
		let (mut c, mut o, a) = optical_fixture(true, false)?;
		let scene = InteractionScene::default();
		for target in [0.1, 0.2, 0.4, 0.7, 1.05, 1.2, 2.4] {
			while c.tick as f64 * DT + 1e-8 < target {
				c.advance_optical(target, Mat4::IDENTITY, &scene, &mut o, &a);
			}
			for id in 0..16 {
				let expected = c.authored_base(c.tick as f64 * DT, id);
				assert_eq!(expected.is_some(), c.bodies[id].is_some());
				if let (Some((p, threshold, seed)), Some(b)) = (expected, &c.bodies[id]) {
					assert_eq!(b.id, c.specs[id].id);
					assert!(b.model().abs_diff_eq(p.transform().to_matrix(), 2e-6));
					assert_eq!(c.retirement(id), (threshold, seed));
					let r = Vec3::new(0.1, 0.5, -0.2);
					let (w, v) =
						crate::parts::combined_choreography::OpticalChoreography::material(b, r);
					assert!(v.is_finite());
					assert!(
						(b.pose
							.coefficients
							.inverse(b.model().inverse().transform_point3(w.as_vec3()))
							- r)
							.length() < 1e-4
					);
				}
			}
			if target >= 1.2 {
				assert!(c.bodies.iter().all(Option::is_none));
			}
		}
		assert!(o.particles.is_empty());
		Ok(())
	}
	#[test]
	fn optical_contact_identical_motion_and_partitioned_secondaries(
	) -> Result<(), Box<dyn std::error::Error>> {
		for landing in [false, true] {
			let end = if landing { 0.8 } else { 3.5 };
			let scene = InteractionScene {
				compact_landing: landing,
				cartoon_wave_timing: !landing,
				..default()
			};
			let (mut reference, _, _) = optical_fixture(landing, true)?;
			while reference.tick as f64 * DT + 1e-8 < end {
				reference.advance(end, Mat4::IDENTITY, &scene);
			}
			let mut results = Vec::new();
			for partition in [0.037f64, 0.113] {
				let (mut c, mut o, a) = optical_fixture(landing, true)?;
				let mut time = 0.;
				while time < end {
					time = (time + partition).min(end);
					while c.tick < (time / DT + 1e-6).floor() as u32 {
						c.advance_optical(time, Mat4::IDENTITY, &scene, &mut o, &a);
					}
				}
				for (x, y) in c.bodies.iter().zip(&reference.bodies) {
					match (x, y) {
						(Some(x), Some(y)) => {
							assert_eq!(x.position, y.position);
							assert_eq!(x.affine, y.affine);
							assert_eq!(x.rotation, y.rotation);
						}
						(None, None) => {}
						_ => return Err("body mismatch".into()),
					}
				}
				let supports = std::array::from_fn(|i| {
					(
						Vec3::from_array(a.manifest.rest_support[i].min),
						Vec3::from_array(a.manifest.rest_support[i].max),
					)
				});
				for p in o.primitives(&c, supports) {
					assert!(p.inverse.is_finite());
					assert!(p.min.cmple(p.max).all());
					assert!(p.masked);
				}
				results.push(
					o.particles
						.iter()
						.map(|p| (p.id, p.position, p.velocity, p.age))
						.collect::<Vec<_>>(),
				);
				assert!(
					o.particles.len()
						<= crate::parts::combined_choreography::MAX_CLOUDS
							+ crate::parts::combined_choreography::MAX_GRAINS
				);
				c.advance_optical(0.2, Mat4::IDENTITY, &scene, &mut o, &a);
				while c.tick as f64 * DT + 1e-8 < 0.2 {
					c.advance_optical(0.2, Mat4::IDENTITY, &scene, &mut o, &a);
				}
				let (mut reset, mut reset_o, ra) = optical_fixture(landing, true)?;
				while reset.tick as f64 * DT + 1e-8 < 0.2 {
					reset.advance_optical(0.2, Mat4::IDENTITY, &scene, &mut reset_o, &ra);
				}
				assert_eq!(
					o.particles.iter().map(|p| p.position).collect::<Vec<_>>(),
					reset_o.particles.iter().map(|p| p.position).collect::<Vec<_>>()
				);
			}
			assert_eq!(results[0], results[1]);
		}
		Ok(())
	}
	#[test]
	fn optical_tail_preserves_field_and_released_particles_without_rebirth(
	) -> Result<(), Box<dyn std::error::Error>> {
		use crate::parts::combined::simulation::Particle;
		for landing in [false, true] {
			let (mut c, mut o, a) = optical_fixture(landing, true)?;
			let (mut old, mut old_o, old_a) = optical_fixture(landing, true)?;
			let end = if landing { 1.2 } else { 4. };
			let start = (end / DT + 1e-6).floor() as u32 - 1;
			c.tick = start;
			old.tick = start;
			let scene = InteractionScene {
				compact_landing: landing,
				cartoon_wave_timing: !landing,
				..default()
			};
			let cloud = Particle {
				id: 1,
				parent: 0,
				region: 0,
				kind: 0,
				variant: 0,
				birth: end - 0.1,
				life: 1.2,
				position: DVec3::new(0.2, 0.4, 0.1),
				velocity: DVec3::new(0.1, 0.05, -0.02),
				radius: 0.03,
				rotation: bevy::math::DQuat::IDENTITY,
				drag: 2.2,
				gravity: 0.,
				base_concentration: 0.4,
				spin: 0.2,
				scale: DVec3::splat(0.03),
				concentration: 0.3,
				age: 0.1,
				color: DVec3::splat(0.3),
			};
			o.particles.push(cloud.clone());
			old_o.particles.push(cloud);
			for _ in 0..24 {
				let next = (c.tick + 1) as f64 * DT;
				c.advance_optical(next, Mat4::IDENTITY, &scene, &mut o, &a);
				old.advance_profiled(next, Mat4::IDENTITY, &scene, None);
				old.bodies.fill(None);
				old_o.step(&old, &scene, old.field.as_ref(), &old_a);
				assert!(c.bodies.iter().all(Option::is_none));
				assert_eq!(
					c.field.as_ref().ok_or("new field")?.steps,
					old.field.as_ref().ok_or("old field")?.steps
				);
				for p in [DVec3::ZERO, DVec3::new(0.2, 0.5, 0.1), DVec3::new(1., 1., -1.)] {
					assert_eq!(
						c.field.as_ref().ok_or("new")?.sample(p),
						old.field.as_ref().ok_or("old")?.sample(p)
					);
				}
				assert_eq!(o.particles.len(), old_o.particles.len());
				for (x, y) in o.particles.iter().zip(&old_o.particles) {
					assert_eq!(x.position, y.position);
					assert_eq!(x.velocity, y.velocity);
					assert_eq!(x.concentration, y.concentration);
				}
			}
		}
		Ok(())
	}
	#[test]
	fn optical_float32_duration_reaches_terminal_tick() -> Result<(), Box<dyn std::error::Error>> {
		for landing in [false, true] {
			let (mut c, mut o, a) = optical_fixture(landing, true)?;
			let duration = o.duration();
			let terminal = (duration as f64 / DT).round() as u32;
			c.tick = terminal - 2;
			c.advance_optical(
				duration as f64,
				Mat4::IDENTITY,
				&InteractionScene::default(),
				&mut o,
				&a,
			);
			assert_eq!(c.tick, terminal);
			assert!(c.tick as f32 / 120. >= duration);
			assert!(c.bodies.iter().all(Option::is_none));
		}
		Ok(())
	}
}

impl CartoonFlow {
	pub(crate) fn advance_optical(
		&mut self,
		target: f64,
		parent: Mat4,
		scene: &InteractionScene,
		optical: &mut crate::parts::combined_choreography::OpticalChoreography,
		asset: &crate::parts::combined::assets::CombinedAsset,
	) {
		optical.body_ns = 0;
		optical.particle_ns = 0;
		optical.steps = 0;
		let duration = optical.duration() as f64;
		// Float32 5.7 is just below tick684. Reach the authored terminal tick
		// without changing the established sub-tick rounding during live motion.
		let goal = if target >= duration {
			(duration / DT).round() as u32
		} else {
			(target / DT + 1e-6).floor() as u32
		};
		if goal < self.tick {
			self.tick = 0;
			self.bodies.fill(None);
			self.history.clear();
			self.contact_sides.fill(0.);
			let initial = self.region.as_ref().map(|r| r.scene(0.));
			self.field = self.contact.then(|| Flow::new(initial.as_ref().unwrap_or(scene).wind));
			optical.particles.clear();
			optical.dropped = [0; 2];
		}
		let end = goal.min(self.tick + 32);
		while self.tick < end {
			let stamp = std::time::Instant::now();
			let next = (self.tick + 1) as f64 * DT;
			let local_scene = self.region.as_ref().map(|r| r.scene(self.tick as f64 * DT));
			let scene = local_scene.as_ref().unwrap_or(scene);
			if self.contact {
				if next < if self.landing { 1.2 } else { 4. } {
					self.advance_profiled(next, parent, scene, None);
				} else {
					// Only the field/secondaries remain. Do not recreate expired bodies
					// from the authored terminal hold and discard them on every tail tick.
					if self.tick % 2 == 0 {
						if let Some(f) = &mut self.field {
							f.step(scene, self.tick as f64 * DT);
						}
					}
					self.tick += 1;
				}
			} else {
				for id in 0..self.count() {
					let Some((pose, _, _)) = self.authored_base(next, id) else {
						self.bodies[id] = None;
						continue;
					};
					let p = Self::world_pose(pose, parent);
					let old = self.bodies[id].take();
					let mut b = Self::initialize(id, p, DVec3::ZERO);
					b.id = self.specs[id].id;
					if let Some(old) = old {
						b.previous = old.pose;
						b.previous_model = old.model();
					} else if let Some((old, _, _)) = self.authored_base(next - DT, id) {
						b.previous = Self::world_pose(old, parent);
						b.previous_model = b.previous.transform().to_matrix();
					}
					b.age = next - b.birth;
					self.bodies[id] = Some(b);
				}
				self.tick += 1;
			}
			if next >= if self.landing { 1.2 } else { 4. } {
				self.bodies.fill(None);
			}
			optical.body_ns += stamp.elapsed().as_nanos() as u64;
			let stamp = std::time::Instant::now();
			optical.step(self, scene, self.field.as_ref(), asset);
			optical.particle_ns += stamp.elapsed().as_nanos() as u64;
			optical.steps += 1;
		}
	}
}
