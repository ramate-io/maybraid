use crate::parts::combined::interaction::*;
#[test]
fn distances_normals_and_surface_velocity() -> Result<(), String> {
	let scene = InteractionScene::default();
	let mut c = scene.capsule(0.);
	c.position = DVec3::ZERO;
	c.linear = DVec3::Z;
	c.angular = DVec3::Y * 2.;
	let q = c.query(DVec3::X * 0.5);
	assert!((q.distance - 0.22).abs() < 1e-12);
	assert_eq!(q.normal, DVec3::X);
	assert!((q.velocity.z - 0.44).abs() < 1e-12);
	let b = scene.boxes()[0];
	assert!(b.query(b.position).distance < 0.);
	assert!((b.query(b.position + DVec3::X * 0.52).distance - 0.2).abs() < 1e-12);
	let velocity = c.remove_inward(DVec3::X * 0.5, -DVec3::X * 2. + DVec3::Y);
	assert!(velocity.x.abs() < 1e-12);
	assert_eq!(velocity.y, 1.);
	Ok(())
}
#[test]
fn fast_pass_and_thin_wall_substeps_do_not_tunnel() -> Result<(), String> {
	let scene = InteractionScene { scenario: Scenario::Static, ..Default::default() };
	let mut p = DVec3::new(1.5, 0.65, -0.75);
	let mut v = DVec3::X * 8.;
	for i in 0..30 {
		(p, v) = scene.integrate(p, v, DVec3::X * 8., 1., 0., 0.004, i as f64 * DT);
		assert!(p.x < 1.75601);
	}
	// The target can accelerate briefly within the10um contact slop.
	assert!(v.x >= 0. && v.x < 0.04);
	let scene = InteractionScene { scenario: Scenario::Fast, ..Default::default() };
	let mut p = DVec3::new(0.25, 0.7, 0.);
	let mut v = DVec3::ZERO;
	for i in 0..100 {
		let t = 2.8 + i as f64 * DT;
		(p, v) = scene.integrate(p, v, DVec3::ZERO, 2., 0., 0.01, t);
		assert!(scene.capsule(t + DT).query(p).distance >= 0.00999);
	}
	assert!(p.z > 0.3);
	Ok(())
}
#[test]
fn teleport_and_pause_placement_have_no_derived_impulse() -> Result<(), String> {
	let mut scene = InteractionScene { scenario: Scenario::Manual, ..Default::default() };
	scene.place(1., DVec3::new(0., 0.7, 1.), true);
	assert_eq!(scene.capsule(0.9).linear, DVec3::ZERO);
	assert_eq!(scene.capsule(1.).linear, DVec3::ZERO);
	scene.place(1., DVec3::new(4., 0.7, 1.), true);
	assert_eq!(scene.capsule(1.).linear, DVec3::ZERO);
	scene.place(2., DVec3::new(5., 0.7, 1.), false);
	assert_eq!(scene.capsule(1.5).linear, DVec3::X);
	scene.reset_proxy(0.);
	assert_eq!(scene.capsule(0.).linear, DVec3::ZERO);
	for i in 1..=MAX_INPUTS + 20 {
		scene.place(i as f64, DVec3::new(i as f64, 0.7, 0.), false);
	}
	assert_eq!(scene.manual.len(), MAX_INPUTS);
	let first = scene.manual.front().ok_or("retained snapshot")?;
	assert_eq!(scene.capsule(first.time - 1.).position, first.position);
	assert_eq!(scene.capsule(first.time - 1.).linear, DVec3::ZERO);
	Ok(())
}
