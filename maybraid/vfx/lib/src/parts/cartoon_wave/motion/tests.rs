use crate::parts::cartoon_wave::{
	motion::{Deformation, Wave},
	tests::rest,
};
use bevy::prelude::*;
use serde_json::Value;
fn vec(v: &Value) -> Result<Vec3, Box<dyn std::error::Error>> {
	Ok(Vec3::new(
		v[0].as_f64().ok_or("x")? as f32,
		v[1].as_f64().ok_or("y")? as f32,
		v[2].as_f64().ok_or("z")? as f32,
	))
}
fn close(a: Vec3, b: Vec3, tolerance: f32) {
	assert!((a - b).abs().max_element() < tolerance, "{a:?} != {b:?}");
}
#[test]
fn delivered_parent_child_states_and_material_velocity() -> Result<(), Box<dyn std::error::Error>> {
	let reference: Value = serde_json::from_str(include_str!("../reference.json"))?;
	let wave = Wave::new(rest()?);
	for sample in reference["samples"].as_array().ok_or("samples")? {
		let t = sample["time"].as_f64().ok_or("time")? as f32;
		let states = wave.states(t);
		let expected = sample["states"].as_array().ok_or("states")?;
		assert_eq!(states.iter().flatten().count(), expected.len(), "t={t}");
		for e in expected {
			let id = e["id"].as_u64().ok_or("id")? as usize;
			let s = states[id].ok_or("missing pose")?;
			close(s.position, vec(&e["position"])?, 0.0001);
			close(s.scale, vec(&e["scale"])?, 0.00001);
			close(s.pivot, vec(&e["pivot"])?, 0.00001);
			assert_eq!(s.variant, e["variant"].as_u64().ok_or("variant")? as usize);
			assert_eq!(s.clip, e["clip"].as_u64().ok_or("clip")? as usize);
			assert!((s.phase - e["phase"].as_f64().ok_or("phase")? as f32).abs() < 0.00001);
			assert!(
				(s.support - e["support_correction_m"].as_f64().ok_or("support")? as f32).abs()
					< 0.00001
			);
			assert!((s.tilt - e["tilt_degrees"].as_f64().ok_or("tilt")? as f32).abs() < 0.00001);
			for j in 0..3 {
				let axis = [Vec3::X, Vec3::Y, Vec3::Z][j];
				let col = Vec3::new(
					e["rotation"][0][j].as_f64().ok_or("r")? as f32,
					e["rotation"][1][j].as_f64().ok_or("r")? as f32,
					e["rotation"][2][j].as_f64().ok_or("r")? as f32,
				);
				close(s.rotation * axis, col, 0.00001);
			}
		}
	}
	for (k, e) in reference["initial"].as_array().ok_or("initial")?.iter().enumerate() {
		close(wave.children[k].anchor, vec(&e["anchor"])?, 0.00001);
		// 1 ms finite differences in float32 amplify rounding; 0.4 mm/s tolerance.
		close(wave.children[k].velocity, vec(&e["velocity"])?, 0.0004);
		close(wave.children[k].parent.scale, vec(&e["scale"])?, 0.00001);
	}
	Ok(())
}
#[test]
fn delivered_deformation_and_world_normal_probes() -> Result<(), Box<dyn std::error::Error>> {
	let r: Value = serde_json::from_str(include_str!("../reference.json"))?;
	let wave = Wave::new(rest()?);
	for p in r["local"].as_array().ok_or("local")? {
		let c = Deformation::at(
			p["phase"].as_f64().ok_or("phase")? as f32,
			p["clip"].as_u64().ok_or("clip")? as usize,
		);
		for i in 0..4 {
			assert!((c.0[i] - p["coefficients"][i].as_f64().ok_or("c")? as f32).abs() < 0.000001);
		}
		close(c.map(Vec3::new(0.2, 0.7, -0.3)), vec(&p["position"])?, 0.000001);
		close(
			c.normal(Vec3::new(0.2, 0.7, -0.3), Vec3::new(0.3, 0.8, -0.2)),
			vec(&p["normal"])?,
			0.0003,
		);
	}
	for p in r["probes"].as_array().ok_or("probes")? {
		let t = p["time_s"].as_f64().ok_or("time")? as f32;
		let id = p["parent"].as_u64().ok_or("parent")? as usize;
		let s = wave.states(t)[id].ok_or("probe pose")?;
		let rest = vec(&p["rest_position_m"])?;
		close(s.world(rest), vec(&p["world_position_m"])?, 0.0001);
		close(s.normal(rest, vec(&p["rest_normal"])?), vec(&p["world_normal"])?, 0.0005);
		close(s.transform().transform_point(s.coefficients.map(rest)), s.world(rest), 0.000001);
	}
	Ok(())
}
#[test]
fn deterministic_partitions_and_exact_deformed_mesh_support(
) -> Result<(), Box<dyn std::error::Error>> {
	let wave = Wave::new(rest()?);
	for t in [2.5, 3.1, 3.57, 4.] {
		for i in [0, 3, 5, 9] {
			let s = wave.parent(t, i).ok_or("late")?;
			let base = crate::parts::cartoon_wave::motion::Pose::base(t, i).ok_or("base")?;
			let points = &wave.rest[s.variant];
			let min = |s: crate::parts::cartoon_wave::motion::Pose| {
				points.iter().map(|&p| s.world(p).y).fold(f32::INFINITY, f32::min)
			};
			assert!((min(s) - min(base)).abs() < 0.000001);
			// Conservative deformed local domain must contain every current vertex.
			for &p in points.iter() {
				let q = s.coefficients.map(p);
				assert!(
					q.cmpge(Vec3::new(-2.042, -0.444, -1.817)).all()
						&& q.cmple(Vec3::new(1.917, 3.531, 1.786)).all()
				);
			}
		}
	}
	let a = (0..400).map(|_| 0.01f32).sum::<f32>();
	let b = (0..100).map(|_| 0.04f32).sum::<f32>();
	for i in 0..15 {
		let x = wave.states(a)[i].ok_or("a")?;
		let y = wave.states(b)[i].ok_or("b")?;
		close(x.world(Vec3::new(0.2, 0.7, -0.3)), y.world(Vec3::new(0.2, 0.7, -0.3)), 0.00002);
	}
	Ok(())
}
