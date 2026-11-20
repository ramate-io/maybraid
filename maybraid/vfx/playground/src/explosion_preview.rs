//! Local parity presentation only; restores the normal playground when leaving these presets.
use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*};
#[derive(Resource)]
struct Saved {
	depth: bool,
	windows: Vec<(Entity, bevy::window::WindowResolution)>,
	controllers: Vec<(Entity, f32, f32, bool)>,
	dither: Vec<(Entity, Option<bevy::core_pipeline::tonemapping::DebandDither>)>,
	parity_ground: Entity,
	cameras: Vec<(Entity, Transform, Projection, Option<Tonemapping>)>,
	clear: Color,
	ground: Vec<(Entity, Handle<StandardMaterial>)>,
	hidden: Vec<(Entity, Visibility)>,
}
pub fn scene(w: &mut World, enabled: bool) {
	if enabled && w.contains_resource::<Saved>() {
		return;
	}
	if !enabled {
		if let Some(saved) = w.remove_resource::<Saved>() {
			let _ = w.despawn(saved.parity_ground);
			for (e, res) in saved.windows {
				if let Some(mut window) = w.get_mut::<Window>(e) {
					window.resolution = res;
				}
			}
			for (e, yaw, pitch, inspection) in saved.controllers {
				if let Some(mut c) = w.get_mut::<crate::camera::CameraController>(e) {
					c.yaw = yaw;
					c.pitch = pitch;
					c.inspection = inspection;
				}
			}
			for (e, d) in saved.dither {
				if let Ok(mut e) = w.get_entity_mut(e) {
					if let Some(d) = d {
						e.insert(d);
					} else {
						e.remove::<bevy::core_pipeline::tonemapping::DebandDither>();
					}
				}
			}

			w.insert_resource(ClearColor(saved.clear));
			for (e, t, p, m) in saved.cameras {
				if let Ok(mut e) = w.get_entity_mut(e) {
					e.insert((t, p));
					if let Some(m) = m {
						e.insert(m);
					} else {
						e.remove::<Tonemapping>();
					}
				}
			}
			for (e, h) in saved.ground {
				if let Ok(mut e) = w.get_entity_mut(e) {
					e.insert(MeshMaterial3d(h));
				}
			}
			for (e, v) in saved.hidden {
				if let Ok(mut e) = w.get_entity_mut(e) {
					e.insert(v);
				}
			}
		}
		return;
	}
	let clear = w.resource::<ClearColor>().0;
	let ground = w
		.query_filtered::<(Entity, &MeshMaterial3d<StandardMaterial>), With<crate::ground::GroundPlane>>(
		)
		.iter(w)
		.map(|(e, m)| (e, m.0.clone()))
		.collect::<Vec<_>>();
	let mut hidden = Vec::new();
	let obstacles = w
		.query_filtered::<(Entity, &Visibility), With<crate::interaction::ReferenceObstacle>>()
		.iter(w)
		.map(|(e, v)| (e, *v))
		.collect::<Vec<_>>();
	for (e, v) in obstacles {
		hidden.push((e, v));
		w.entity_mut(e).insert(Visibility::Hidden);
	}
	let h = w.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
		base_color: Color::srgb_u8(122, 138, 90),
		unlit: true,
		..default()
	});
	for (e, _) in &ground {
		let v = w.get::<Visibility>(*e).copied().unwrap_or(Visibility::Visible);
		hidden.push((*e, v));
		w.entity_mut(*e).insert(Visibility::Hidden);
	}
	let mesh = w.resource_mut::<Assets<Mesh>>().add(Circle::new(40.).mesh().resolution(96));
	let parity_ground = w
		.spawn((
			Mesh3d(mesh),
			MeshMaterial3d(h),
			Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
		))
		.id();
	let cameras=w.query_filtered::<(Entity,&Transform,&Projection,Option<&Tonemapping>),With<crate::camera::CameraController>>().iter(w).map(|(e,t,p,m)|(e,*t,p.clone(),m.copied())).collect();
	let windows = w
		.query::<(Entity, &Window)>()
		.iter(w)
		.map(|(e, w)| (e, w.resolution.clone()))
		.collect();
	let controllers = w
		.query::<(Entity, &crate::camera::CameraController)>()
		.iter(w)
		.map(|(e, c)| (e, c.yaw, c.pitch, c.inspection))
		.collect();
	let dither=w.query_filtered::<(Entity,Option<&bevy::core_pipeline::tonemapping::DebandDither>),With<crate::camera::CameraController>>().iter(w).map(|(e,d)|(e,d.copied())).collect();
	w.insert_resource(Saved {
		clear,
		ground,
		hidden,
		cameras,
		depth: false,
		parity_ground,
		windows,
		controllers,
		dither,
	});
	w.insert_resource(ClearColor(Color::srgb_u8(128, 128, 128)));
	view(w, "explosion-front");
}
pub fn view(w: &mut World, name: &str) {
	if let Some(mut saved) = w.get_resource_mut::<Saved>() {
		saved.depth = name == "explosion-depth";
	}
	let target = Vec3::new(0., 4.8, 0.);
	let elevation = 20f32.to_radians();
	let azimuth = if name == "explosion-three-quarter" { 45f32.to_radians() } else { 0. };
	let mut eye = target
		+ Vec3::new(
			azimuth.sin() * elevation.cos(),
			elevation.sin(),
			azimuth.cos() * elevation.cos(),
		) * 18.;
	let mut aim = target;
	let mut fov = 2. * (5.4f32 / 18.).atan();
	if name == "explosion-close" {
		eye = Vec3::new(4., 3., 5.);
		aim = Vec3::new(0., 2.8, 0.);
	}
	if name == "explosion-inside" {
		eye = Vec3::new(0.1, 2.4, 0.1);
		aim = eye + Vec3::Z;
		fov = 60f32.to_radians();
	}
	let ids = w
		.query_filtered::<Entity, With<crate::camera::CameraController>>()
		.iter(w)
		.collect::<Vec<_>>();
	for id in ids {
		let t = Transform::from_translation(eye).looking_at(aim, Vec3::Y);
		let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
		w.entity_mut(id).insert((
			t,
			Tonemapping::None,
			bevy::core_pipeline::tonemapping::DebandDither::Disabled,
			Projection::Perspective(PerspectiveProjection {
				fov,
				near: 0.02,
				far: 400.,
				..default()
			}),
		));
		if let Some(mut c) = w.get_mut::<crate::camera::CameraController>(id) {
			c.yaw = yaw;
			c.pitch = pitch;
			c.inspection = true;
		}
	}
	w.resource_mut::<crate::commands::CameraOrbit>().enabled = false;
	if name == "explosion-depth" {
		let ids = w
			.query_filtered::<Entity, With<crate::interaction::ReferenceObstacle>>()
			.iter(w)
			.collect::<Vec<_>>();
		for e in ids {
			w.entity_mut(e).insert(Visibility::Visible);
		}
	}
	for mut window in w.query::<&mut Window>().iter_mut(w) {
		window.resolution.set_scale_factor_override(Some(1.));
		window.resolution.set_physical_resolution(1280, 720);
	}
}

pub mod profiling;

#[derive(Component)]
pub struct ReadbackTimer {
	pub path: std::path::PathBuf,
	pub remaining: f32,
}
pub fn readback(
	mut c: Commands,
	time: Res<Time<Real>>,
	mut q: Query<(Entity, &mut ReadbackTimer)>,
) {
	for (e, mut r) in &mut q {
		r.remaining -= time.delta_secs();
		if r.remaining <= 0. {
			c.insert_resource(maybraid_vfx::parts::smoke_explosion::render::DensityReadback(
				r.path.clone(),
			));
			c.entity(e).try_despawn();
		}
	}
}

pub fn enforce_scene(w: &mut World) {
	let Some(saved) = w.get_resource::<Saved>() else {
		return;
	};
	let depth = saved.depth;
	let ids=w.query_filtered::<Entity,Or<(With<crate::interaction::Proxy>,With<crate::interaction::ReferenceObstacle>)>>().iter(w).collect::<Vec<_>>();
	for e in ids {
		let v = if depth { Visibility::Visible } else { Visibility::Hidden };
		if let Some(mut current) = w.get_mut::<Visibility>(e) {
			if *current != v {
				*current = v;
			}
		}
	}
}
