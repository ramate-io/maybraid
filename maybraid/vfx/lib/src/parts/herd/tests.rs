use crate::parts::herd::*;
use crate::{
	composition::{EffectDefinition, EffectLayer, EffectPart},
	lifecycle::{gate_vfx_instances, tick_vfx_instances},
	spawn::spawn_vfx,
};
use bevy::{
	asset::{AssetApp, AssetPlugin},
	time::TimeUpdateStrategy,
};
use std::time::Duration;
fn input(tick: u32, style: Style, emitting: bool) -> Frame {
	let t = tick as f64 * DT;
	Frame {
		heights: Vec::new(),
		time: t,
		emitters: vec![EmitterInput {
			id: 0,
			position: DVec3::new(t * 0.6, 0., 0.),
			velocity: DVec3::X * 0.6,
			ground: default(),
			discontinuous: tick == 1,
		}],
		colliders: vec![],
		births: vec![],
		style,
		emitting,
	}
}
fn state(world: &mut World, capacity: usize) -> HerdState {
	let scene =
		InteractionScene { external: Some(vec![]), wind: DVec3::new(0.06, 0., 0.), ..default() };
	HerdState {
		part: HerdPart {
			meshes: default(),
			assets: default(),
			settings: Settings { spacing: 0.2, max_puffs: capacity, ..default() },
		},
		tick: 0,
		ready: true,
		field: Flow::new(scene.wind),
		slots: (0..capacity)
			.map(|_| Slot {
				entity: world.spawn_empty().id(),
				material: default(),
				mesh: default(),
				noise: Vec::new(),
				nodes: Vec::new(),
				drawn: None,
				puff: None,
			})
			.collect(),
		emitters: vec![],
		fine: Vec::with_capacity(128),
		scene,
		group: default(),
		history: VecDeque::new(),
		rejected: 0,
		accepted: 0,
		rebuilds: 0,
		geometry_bytes: 0,
		max_active: 0,
		terrain_misses: 0,
		tiles: [None; 4],
		tile_terrain: std::array::from_fn(|_| None),
	}
}
#[test]
fn styles_share_births_world_release_and_bounded_retirement() -> Result<(), String> {
	let fields = ClosedFields::fixture(&mut Assets::default());
	let mut world = World::new();
	let mut a = state(&mut world, 4);
	let mut b = state(&mut world, 4);
	for tick in 1..=240 {
		a.step(input(tick, Style::Cartoon, tick < 150), &fields, 13, None);
		b.step(input(tick, Style::Combined, tick < 150), &fields, 13, None);
		assert_eq!(a.accepted, b.accepted);
		assert_eq!(a.rejected, b.rejected);
		for (a, b) in a.slots.iter().zip(&b.slots) {
			if let (Some(a), Some(b)) = (&a.puff, &b.puff) {
				assert_eq!(a.seed, b.seed);
				assert_eq!(a.body.position, b.body.position);
				assert_eq!(a.body.affine, b.body.affine);
				assert!(a.body.position.is_finite());
				assert!(a.body.affine.determinant() > 0.);
			}
		}
		assert!(a.active() <= 4);
		let (fine, grains) = b.fine_counts();
		assert!(fine <= MAX_FINE && grains <= MAX_GRAINS);
	}
	assert!(a.accepted > 0);
	assert_eq!(a.fine_counts(), (0, 0));
	let born = a.accepted;
	for tick in 241..=480 {
		a.step(input(tick, Style::Cartoon, false), &fields, 13, None);
		b.step(input(tick, Style::Combined, false), &fields, 13, None);
	}
	assert_eq!(a.accepted, born);
	assert_eq!(a.active(), 0);
	assert_eq!(b.active(), 0);
	assert_eq!(b.fine_counts(), (0, 0));
	Ok(())
}
#[test]
fn input_bounds_and_monotonicity() -> Result<(), String> {
	let mut q = Inputs::default();
	q.push(input(1, Style::Cartoon, true))?;
	assert!(q.push(input(1, Style::Cartoon, true)).is_err());
	let mut bad = input(2, Style::Cartoon, true);
	bad.emitters[0].position.x = f64::NAN;
	assert!(q.push(bad).is_err());
	let mut bad = input(2, Style::Cartoon, true);
	bad.emitters = vec![bad.emitters[0]; 21];
	assert!(q.push(bad).is_err());
	Ok(())
}
#[test]
fn host_pause_resume_cancel_owned_resources() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	app.init_asset::<Mesh>().init_asset::<HerdMaterial>();
	let fields = ClosedFields::fixture(app.world_mut().resource_mut::<Assets<Mesh>>().as_mut());
	let handles = std::array::from_fn(|i| fields.fields[i].base.clone());
	app.insert_resource(fields)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(DT)))
		.add_plugins(HerdPlugin)
		.add_systems(Update, tick_vfx_instances)
		.add_systems(PostUpdate, gate_vfx_instances);
	let definition = EffectDefinition::new(
		"herd-test",
		[EffectLayer {
			part: EffectPart::Herd(HerdPart {
				meshes: handles,
				assets: default(),
				settings: Settings { max_puffs: 2, ..default() },
			}),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}],
	);
	let mut queue = bevy::ecs::world::CommandQueue::default();
	let mut commands = Commands::new(&mut queue, app.world());
	let root = spawn_vfx(&mut commands, &definition, VfxSpawn { seed: Some(13), ..default() });
	commands.entity(root).insert(Inputs::default());
	queue.apply(app.world_mut());
	app.update();
	app.update();
	assert!(app.world().get::<VfxInstance>(root).ok_or("missing host")?.armed);
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	let before = app.world().get::<VfxInstance>(root).ok_or("missing host")?.age;
	app.update();
	assert_eq!(before, app.world().get::<VfxInstance>(root).ok_or("missing host")?.age);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	let unrelated = app.world_mut().spawn_empty().id();
	app.world_mut().despawn(root);
	app.update();
	assert_eq!(app.world_mut().query::<&HerdState>().iter(app.world()).count(), 0);
	assert_eq!(app.world_mut().query::<&VfxMemberOf>().iter(app.world()).count(), 0);
	assert!(app.world().get_entity(unrelated).is_ok());
	Ok(())
}
#[test]
fn conservative_bounds_cover_deformation_and_affine() -> Result<(), String> {
	let fields = ClosedFields::fixture(&mut Assets::default());
	for phase in [0., 0.4, 1.3, 2.7] {
		for variant in 0..3 {
			let pose = Pose {
				position: Vec3::new(0.2, 0.1, -0.4),
				rotation: Quat::from_rotation_y(1.1),
				scale: Vec3::splat(0.2),
				pivot: Vec3::ZERO,
				coefficients: Deformation::at(phase, 0),
				variant,
				phase,
				clip: 0,
				birth: 0.,
				yaw: 0.,
				support: 0.,
				tilt: 0.,
			};
			let correction = Mat4::from_mat3(Mat3::from_cols(
				Vec3::new(1., 0.12, 0.),
				Vec3::new(0.15, 1., 0.02),
				Vec3::new(0., 0., 0.9),
			));
			let f = &fields.fields[variant];
			let (lo, hi) = f.bounds_for(Pose {
				position: Vec3::ZERO,
				rotation: Quat::IDENTITY,
				scale: Vec3::ONE,
				..pose
			});
			let (lo, hi) = Primitive::transformed_bounds(correction, lo, hi);
			for r in &f.nodes {
				let p = correction.transform_point3(pose.coefficients.map(*r));
				assert!(
					p.cmpge(lo - Vec3::splat(1e-5)).all() && p.cmple(hi + Vec3::splat(1e-5)).all()
				);
			}
		}
	}
	Ok(())
}
#[test]
fn external_culling_matches_the_full_approved_field() -> Result<(), String> {
	let original = InteractionScene::default();
	let mut external = original.clone();
	let mut a = Flow::new(original.wind);
	let mut b = Flow::new(original.wind);
	for frame in 0..8 {
		let t = 2.8 + frame as f64 / 60.;
		external.external =
			Some(original.colliders(t).into_iter().take(original.collider_count()).collect());
		a.step(&original, t);
		b.step(&external, t);
		assert_eq!(a.solid, b.solid);
		assert_eq!(a.velocity, b.velocity);
		assert_eq!(a.pressure, b.pressure);
	}
	Ok(())
}

#[test]
fn combined_routes_same_bodies_with_deformed_density_bounds() -> Result<(), String> {
	let fields = ClosedFields::fixture(&mut Assets::default());
	let mut world = World::new();
	let mut s = state(&mut world, 32);
	let supports = [(-Vec3::splat(2.), Vec3::splat(3.)); 3];
	for i in 0..32 {
		assert!(s.birth(
			Birth {
				time: 0.,
				position: DVec3::new(0.4, 0., 0.),
				normal: DVec3::Y,
				velocity: DVec3::ZERO,
				seed: i
			},
			if i % 2 == 0 { Style::Combined } else { Style::Cartoon },
			&fields,
			1.,
			None
		));
	}
	let ps = s.optical_primitives(supports);
	assert_eq!(ps.len(), 16);
	for p in ps {
		assert!(p.masked);
		assert_eq!(p.parameters.w, 0.);
		assert!(p.parameters.z > 0.);
		assert!(p.inverse.is_finite());
		let b = s
			.slots
			.iter()
			.filter_map(|s| s.puff.as_ref())
			.find(|b| b.style == Style::Combined && b.body.model().inverse() == p.inverse)
			.ok_or("body missing")?;
		for x in [-1., 0., 1.] {
			let rest = Vec3::new(x, 0.5, x);
			let w = b.body.model().transform_point3(b.body.pose.coefficients.map(rest));
			assert!(w.cmpge(p.min).all() && w.cmple(p.max).all());
		}
	}
	s.part.settings.body_density = 0.;
	assert!(s.optical_primitives(supports).iter().all(|p| p.parameters.z == 0.));
	Ok(())
}

#[test]
fn moving_world_settles_drains_and_keeps_released_identity() -> Result<(), String> {
	use std::sync::Arc;
	let fields = ClosedFields::fixture(&mut Assets::default());
	let mut w = World::new();
	let mut s = state(&mut w, 8);
	s.part.settings.travelling = true;
	s.field.rolling = true;
	s.scene.ground_floor = None;
	let h = Arc::new(terrain::Heightfield::new(
		bevy::math::DVec2::new(10., -25.),
		0.5,
		[65, 65],
		1,
		(0..65 * 65).map(|i| 4. + 0.03 * (10. + (i % 65) as f64 * 0.5 - 20.)).collect(),
		vec![true; 65 * 65],
	)?);
	s.scene.terrain = Some(h);
	let mut recorded = None;
	let mut settled = false;
	for tick in 1..=360 {
		let mut f = input(tick, Style::Combined, tick < 100);
		f.emitters[0].position += DVec3::new(20., 4., -15.);
		if tick >= 100 {
			f.emitters[0].position += DVec3::X * 10.;
			f.emitters[0].discontinuous = tick == 100;
		}
		s.step(f, &fields, 13, None);
		if tick == 90 {
			if let Some(p) = s.slots.iter().filter_map(|s| s.puff.as_ref()).next() {
				recorded = Some((p.seed, p.body.position));
			}
		}
		if tick == 110 {
			if let Some((seed, old)) = recorded {
				if let Some(p) =
					s.slots.iter().filter_map(|s| s.puff.as_ref()).find(|p| p.seed == seed)
				{
					assert!(p.body.position.distance(old) < 1.);
					settled = true;
				}
			}
		}
		for p in s.slots.iter().filter_map(|s| s.puff.as_ref()) {
			let b = &p.body;
			for r in fields.fields[b.pose.variant].support.iter() {
				let q = b.model().transform_point3(b.pose.coefficients.map(*r));
				let g = s
					.scene
					.terrain
					.as_ref()
					.and_then(|h| h.sample(q.as_dvec3()))
					.ok_or("support terrain")?;
				assert!(q.y as f64 > g.point.y - 0.002);
			}
		}
		for p in &s.fine {
			let g = p.ground.ok_or("fine terrain")?;
			assert!((p.position - g.point).dot(g.normal) >= -0.002);
			assert!(p.position.is_finite());
		}
	}
	assert!(settled);
	assert!(s.field.shifts > 0);
	assert_eq!(s.active(), 0);
	assert_eq!(s.fine.len(), 0);
	assert!(s.tiles.iter().all(Option::is_none));
	Ok(())
}

#[test]
fn world_rate_conversion_keeps_age_positions_and_dimensions() -> Result<(), String> {
	let f = input(1, Style::Combined, true);
	let old = f.clone();
	let scaled = f.from_world_rates(2.)?;
	assert_eq!(scaled.time, old.time);
	assert_eq!(scaled.emitters[0].position, old.emitters[0].position);
	assert_eq!(scaled.emitters[0].velocity, old.emitters[0].velocity / 2.);
	assert!(old.from_world_rates(0.).is_err());
	Ok(())
}
#[test]
fn terrain_birth_coverage_budget_and_unsupported_descent() -> Result<(), String> {
	let fields = ClosedFields::fixture(&mut Assets::default());
	let mut w = World::new();
	let mut s = state(&mut w, 8);
	s.part.settings.travelling = true;
	s.part.settings.strength = 0.;
	s.scene.ground_floor = None;
	s.field.rolling = true;
	s.scene.terrain = Some(std::sync::Arc::new(terrain::Heightfield::new(
		bevy::math::DVec2::new(-10., -10.),
		0.5,
		[65, 65],
		1,
		vec![4.; 4225],
		vec![true; 4225],
	)?));
	s.field.shift_to(DVec3::new(-2., 3.5, -3.));
	let birth = Birth {
		time: 0.,
		position: DVec3::new(0., 5., 0.),
		normal: DVec3::Y,
		velocity: DVec3::ZERO,
		seed: 13,
	};
	assert!(s.birth(birth, Style::Combined, &fields, 1., None));
	let initial = s.slots[0].puff.as_ref().ok_or("puff")?.body.position.y;
	let mut f = input(1, Style::Combined, false);
	f.emitters.clear();
	for i in 1..=55 {
		f.time = i as f64 * DT;
		s.step(f.clone(), &fields, 13, None);
	}
	let b = &s.slots[0].puff.as_ref().ok_or("live puff")?.body;
	assert!(b.position.y < initial - 0.001);
	assert!(b.velocity.y >= -s.part.settings.settling_speed - 1e-8);
	let mut missing = birth;
	missing.position.x = 100.;
	assert!(!s.birth(missing, Style::Combined, &fields, 1., None));
	// Four retained world tiles are a strict admission limit, never an eviction policy.
	for i in 0..4 {
		s.tiles[i] = Some([i as i64, 0, 0]);
	}
	let old = s.slots[0].puff.as_ref().ok_or("puff")?.seed;
	assert!(!s.birth(
		Birth { position: DVec3::new(0., 5., -1.), seed: 99, ..birth },
		Style::Combined,
		&fields,
		1.,
		None
	));
	assert_eq!(s.slots[0].puff.as_ref().ok_or("retained")?.seed, old);
	Ok(())
}

#[test]
fn sloped_terrain_reconstruction_remains_closed_and_above_support() -> Result<(), String> {
	let fields = ClosedFields::fixture(&mut Assets::default());
	let mut w = World::new();
	let mut s = state(&mut w, 1);
	assert!(s.birth(
		Birth {
			time: 0.,
			position: DVec3::ZERO,
			normal: DVec3::Y,
			velocity: DVec3::ZERO,
			seed: 13
		},
		Style::Cartoon,
		&fields,
		1.,
		None
	));
	let b = &mut s.slots[0].puff.as_mut().ok_or("body")?.body;
	b.position = DVec3::new(20., 4.04, -15.);
	b.scale = DVec3::splat(0.5);
	b.update_pose();
	let terrain = std::sync::Arc::new(terrain::Heightfield::new(
		bevy::math::DVec2::new(18., -17.),
		0.25,
		[17, 17],
		1,
		(0..289).map(|i| 4. + 0.08 * (-2. + (i % 17) as f64 * 0.25)).collect(),
		vec![true; 289],
	)?);
	let scene = InteractionScene {
		external: Some(vec![]),
		terrain: Some(terrain.clone()),
		ground_floor: None,
		..default()
	};
	let f = &fields.fields[b.pose.variant];
	let nodes = f.nodes.iter().map(|p| b.pose.coefficients.map(*p)).collect::<Vec<_>>();
	let mesh = f.rebuild(b, &scene, 0., -1., None, &nodes);
	let positions = mesh
		.attribute(Mesh::ATTRIBUTE_POSITION)
		.and_then(|p| p.as_float3())
		.ok_or("positions")?;
	let indices = mesh.indices().ok_or("indices")?.iter().collect::<Vec<_>>();
	assert!(!indices.is_empty());
	let mut edges = std::collections::HashMap::<([i32; 3], [i32; 3]), (u32, i32)>::new();
	for tri in indices.chunks_exact(3) {
		let p = tri.iter().map(|i| Vec3::from_array(positions[*i])).collect::<Vec<_>>();
		assert!((p[1] - p[0]).cross(p[2] - p[0]).length_squared() > 1e-18);
		let keys = p
			.iter()
			.map(|v| v.to_array().map(|x| (x * 1e6).round() as i32))
			.collect::<Vec<_>>();
		for (a, c) in [(0, 1), (1, 2), (2, 0)] {
			let (a, c) = (keys[a], keys[c]);
			let e = edges.entry((a.min(c), a.max(c))).or_default();
			e.0 += 1;
			e.1 += if a < c { 1 } else { -1 };
		}
		for p in p {
			let q = b.model().transform_point3(b.pose.coefficients.map(p)).as_dvec3();
			let g = terrain.sample(q).ok_or("terrain")?;
			assert!(q.y >= g.point.y - 0.008);
		}
	}
	assert!(edges.values().all(|e| *e == (2, 0)));
	Ok(())
}
