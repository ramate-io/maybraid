//! Bounded opaque isosurfaces. Shared authored body fields; no volume integration.
use crate::parts::cartoon_flow::diagnostics::{timed, Probe};
use crate::parts::{body_flow::BodyMotion, combined::interaction::InteractionScene};
use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};
use std::{collections::HashSet, sync::Arc};
#[derive(Default)]
struct Scratch {
	gradient: Vec<Vec3>,
	positions: Vec<[f32; 3]>,
	normals: Vec<[f32; 3]>,
	edges: Vec<Option<(Vec3, Vec3)>>,
	vertices: Vec<u32>,
	indices: Vec<u32>,
}
impl Scratch {
	fn new() -> Self {
		Self {
			gradient: Vec::with_capacity(COUNT),
			positions: Vec::with_capacity(COUNT),
			normals: Vec::with_capacity(COUNT),
			edges: Vec::with_capacity(COUNT * 7),
			vertices: Vec::with_capacity(COUNT * 7),
			indices: Vec::with_capacity(COUNT * 3),
		}
	}
}
thread_local! { static VALUES: std::cell::RefCell<Vec<f32>> = const { std::cell::RefCell::new(Vec::new()) }; static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::new()); }
pub const N: usize = 20;
const COUNT: usize = N * N * N;
#[derive(Resource, Default)]
pub(crate) struct ClosedFields {
	pub fields: Vec<ClosedField>,
}
pub(crate) struct ClosedField {
	pub nodes: Vec<Vec3>,
	values: Vec<f32>,
	skin: Vec<f32>,
	pub base: Handle<Mesh>,
	pub support: Arc<[Vec3]>,
	lo: Vec3,
	hi: Vec3,
	shape_lo: Vec3,
	shape_hi: Vec3,
}
impl ClosedFields {
	#[cfg(test)]
	pub(crate) fn fixture(meshes: &mut Assets<Mesh>) -> Self {
		let mut fields = Vec::new();
		for _ in 0..3 {
			let lo = Vec3::new(-1.2, -0.2, -1.2);
			let hi = Vec3::new(1.2, 1.4, 1.2);
			let mut nodes = Vec::new();
			let mut values = Vec::new();
			for z in 0..N {
				for y in 0..N {
					for x in 0..N {
						let p = lo
							+ (hi - lo) * Vec3::new(x as f32, y as f32, z as f32) / (N - 1) as f32;
						nodes.push(p);
						values.push(0.5 - p.distance(Vec3::new(0., 0.6, 0.)));
					}
				}
			}
			let mesh = ClosedField::triangulate(&nodes, &values, (hi - lo) / (N - 1) as f32);
			let support: Arc<[Vec3]> =
				nodes.iter().copied().filter(|p| p.y < 0.65).collect::<Vec<_>>().into();
			fields.push(ClosedField {
				skin: vec![0.02; COUNT],
				nodes,
				values,
				base: meshes.add(mesh),
				support,
				lo,
				hi,
				shape_lo: Vec3::new(-0.5, 0.1, -0.5),
				shape_hi: Vec3::new(0.5, 1.1, 0.5),
			});
		}
		Self { fields }
	}

	pub fn prepare(&mut self, meshes: &mut Assets<Mesh>, handles: &[Handle<Mesh>; 3]) -> bool {
		if self.fields.len() == 3 {
			return true;
		}
		let sources = handles
			.iter()
			.map(|h| {
				meshes.get(h).and_then(|mesh| {
					let source =
						crate::parts::cartoon_flow::distance::DistanceMesh::from_mesh(mesh)?;
					let points = mesh
						.attribute(Mesh::ATTRIBUTE_POSITION)?
						.as_float3()?
						.iter()
						.map(|v| Vec3::from_array(*v))
						.collect::<Vec<_>>();
					Some((source, points, h.clone()))
				})
			})
			.collect::<Option<Vec<_>>>();
		let Some(sources) = sources else {
			return false;
		};
		let mut prepared = Vec::with_capacity(3);
		for (source, original_points, source_handle) in sources {
			let (shape_lo, shape_hi) = source.bounds();
			let lo = shape_lo - Vec3::splat(0.16);
			let hi = shape_hi + Vec3::splat(0.16);
			let mut nodes = Vec::with_capacity(COUNT);
			let mut values = Vec::with_capacity(COUNT);
			for z in 0..N {
				for y in 0..N {
					for x in 0..N {
						let p = lo
							+ (hi - lo) * Vec3::new(x as f32, y as f32, z as f32) / (N - 1) as f32;
						nodes.push(p);
						values.push(source.sample(p));
					}
				}
			}
			let skin = nodes
				.iter()
				.map(|p| 0.018 + 0.008 * (0.5 + 0.5 * (p.x * 3.1 + p.y * 2.3 + p.z * 4.7).sin()))
				.collect();
			let mesh = ClosedField::triangulate(&nodes, &values, (hi - lo) / (N - 1) as f32);
			let mut points = mesh
				.attribute(Mesh::ATTRIBUTE_POSITION)
				.and_then(|a| a.as_float3())
				.map(|p| p.iter().map(|v| Vec3::from_array(*v)).collect::<Vec<_>>())
				.unwrap_or_default();
			points.extend(original_points);
			let mut seen = HashSet::new();
			points.retain(|p| seen.insert(p.to_array().map(f32::to_bits)));
			let base = source_handle;
			let mut indices = HashSet::new();
			for clip in 0..3 {
				for phase in 0..7 {
					let deformation = crate::parts::cartoon_wave::motion::Deformation::at(
						phase as f32 * 0.5,
						clip,
					);
					let deformed = points.iter().map(|p| deformation.map(*p)).collect::<Vec<_>>();
					for x in -8..=8 {
						for z in -8..=8 {
							let n = Vec3::new(x as f32 * 0.25, 1., z as f32 * 0.25);
							if let Some((i, _)) = deformed
								.iter()
								.enumerate()
								.min_by(|(_, a), (_, b)| a.dot(n).total_cmp(&b.dot(n)))
							{
								indices.insert(i);
							}
						}
					}
				}
			}
			let mut indices = indices.into_iter().collect::<Vec<_>>();
			indices.sort_unstable();
			let support = indices.into_iter().map(|i| points[i]).collect::<Vec<_>>();
			prepared.push(ClosedField {
				nodes,
				values,
				skin,
				base,
				support: support.into(),
				lo,
				hi,
				shape_lo,
				shape_hi,
			});
		}
		self.fields = prepared;
		true
	}
}
impl ClosedField {
	pub(crate) fn bounds_for(
		&self,
		pose: crate::parts::cartoon_wave::motion::Pose,
	) -> (Vec3, Vec3) {
		pose.bounds(self.lo, self.hi)
	}
	fn intersect(a: f32, b: f32, k: f32) -> f32 {
		let h = (1. - (a - b).abs() / k).max(0.);
		a.min(b) - k * h * h * 0.25
	}

	pub fn needs_contact(&self, b: &BodyMotion, scene: &InteractionScene, t: f64) -> bool {
		let (lo, hi) = b.pose.bounds(self.shape_lo, self.shape_hi);
		let c = b.position.as_vec3();
		let affine =
			Mat4::from_translation(c) * Mat4::from_mat3(b.affine) * Mat4::from_translation(-c);
		let (lo, hi) =
			crate::parts::combined::render::Primitive::transformed_bounds(affine, lo, hi);
		let center = (lo + hi) * 0.5;
		let radius = (hi - lo).length() * 0.5;
		scene.colliders(t).iter().take(scene.collider_count()).any(|v| {
			use crate::parts::combined::interaction::Shape;
			match v.shape {
				Shape::Capsule { radius: r, half_segment: h } => {
					if v.rotation.x.abs() + v.rotation.z.abs() > 1e-6 {
						return v.query(center.as_dvec3()).distance < (radius + 0.04) as f64;
					}
					let p = v.position.as_vec3();
					let h = h as f32;
					let d = Vec3::new(
						(lo.x - p.x).max(p.x - hi.x).max(0.),
						(lo.y - p.y - h).max(p.y - h - hi.y).max(0.),
						(lo.z - p.z).max(p.z - hi.z).max(0.),
					);
					d.length_squared() < (r as f32 + 0.04).powi(2)
				}
				Shape::Box { half } => {
					let m =
						Mat4::from_rotation_translation(v.rotation.as_quat(), v.position.as_vec3());
					let (a, z) = crate::parts::combined::render::Primitive::transformed_bounds(
						m,
						-half.as_vec3(),
						half.as_vec3(),
					);
					lo.cmple(z + Vec3::splat(0.04)).all() && hi.cmpge(a - Vec3::splat(0.04)).all()
				}
			}
		})
	}
	#[cfg(test)]
	pub fn rebuild(
		&self,
		b: &BodyMotion,
		scene: &InteractionScene,
		t: f64,
		threshold: f32,
		noise: Option<&[f32]>,
		deformed: &[Vec3],
	) -> Mesh {
		self.rebuild_profiled(b, scene, t, threshold, noise, deformed, None)
	}
	pub fn rebuild_profiled(
		&self,
		b: &BodyMotion,
		scene: &InteractionScene,
		t: f64,
		threshold: f32,
		noise: Option<&[f32]>,
		deformed: &[Vec3],
		mut profile: Option<&mut Probe>,
	) -> Mesh {
		VALUES.with(|storage| {
			let mut values = storage.borrow_mut();
			let setup_stamp = profile.as_ref().map(|_| std::time::Instant::now());
			let model = b.model();
			let colliders = scene.colliders(t);
			let solids = colliders
				.iter()
				.take(scene.collider_count())
				.map(|c| {
					let inverse =
						Mat4::from_rotation_translation(c.rotation.as_quat(), c.position.as_vec3())
							.inverse();
					let shape = match c.shape {
						crate::parts::combined::interaction::Shape::Capsule {
							radius,
							half_segment,
						} => Vec4::new(radius as f32, half_segment as f32, 0., 0.),
						crate::parts::combined::interaction::Shape::Box { half } => {
							half.as_vec3().extend(1.)
						}
					};
					(inverse, shape)
				})
				.collect::<Vec<_>>();
			values.clone_from(&self.values);
			if let (Some(p), Some(t)) = (profile.as_deref_mut(), setup_stamp) {
				p.frame.ns[crate::parts::cartoon_flow::diagnostics::Stage::ScalarSetup as usize] +=
					t.elapsed().as_nanos() as u64;
			}
			let scalar_stamp = profile.as_ref().map(|_| std::time::Instant::now());
			for (i, value) in values.iter_mut().enumerate() {
				let world = model.transform_point3(deformed[i]);
				if let Some(g) = scene.terrain.as_ref().and_then(|h| h.sample(world.as_dvec3())) {
					let d = (world.y - g.point.y as f32) * g.normal.y as f32;
					*value = Self::intersect(
						*value,
						(d - self.skin[i]) / b.scale.max_element().max(0.01) as f32,
						0.08,
					);
				}
				for (inverse, shape) in &solids {
					let q = inverse.transform_point3(world);
					let d = if shape.w < 0.5 {
						(q - Vec3::new(0., q.y.clamp(-shape.y, shape.y), 0.)).length() - shape.x
					} else {
						let v = q.abs() - shape.truncate();
						v.max(Vec3::ZERO).length() + v.max_element().min(0.)
					};
					*value = Self::intersect(
						*value,
						(d - self.skin[i]) / b.scale.max_element().max(0.01) as f32,
						0.08,
					);
				}
				if threshold >= 0. {
					*value = value.min((noise.map_or(1., |n| n[i]) - threshold) * 0.35);
				}
			}

			if let (Some(p), Some(t)) = (profile.as_deref_mut(), scalar_stamp) {
				p.frame.ns[crate::parts::cartoon_flow::diagnostics::Stage::Scalar as usize] +=
					t.elapsed().as_nanos() as u64;
			}
			Self::triangulate_profiled(
				&self.nodes,
				&values,
				(self.hi - self.lo)
					/ ((self.nodes.len() as f64).cbrt().round() as usize - 1) as f32,
				profile,
			)
		})
	}
	pub fn fill_noise(&self, seed: u32, output: &mut Vec<f32>) {
		output.clear();
		output.extend(self.nodes.iter().map(|p| Self::noise(*p, seed)));
	}
	pub fn noise_grid(&self, seed: u32) -> Vec<f32> {
		self.nodes.iter().map(|p| Self::noise(*p, seed)).collect()
	}
	pub(crate) fn noise(p: Vec3, seed: u32) -> f32 {
		fn hash(p: IVec3, seed: u32) -> f32 {
			let q = p.as_uvec3();
			let mut h = q.x.wrapping_mul(0x8da6b343)
				^ q.y.wrapping_mul(0xd8163841)
				^ q.z.wrapping_mul(0xcb1ab31f)
				^ seed;
			h ^= h >> 16;
			h = h.wrapping_mul(0x7feb352d);
			h ^= h >> 15;
			h = h.wrapping_mul(0x846ca68b);
			h ^= h >> 16;
			(h & 0xffffff) as f32 / 16777215.
		}
		fn value(p: Vec3, seed: u32) -> f32 {
			let c = p.floor().as_ivec3();
			let t = p - p.floor();
			let f = t * t * t * (t * (t * 6. - Vec3::splat(15.)) + Vec3::splat(10.));
			let mut out = 0.;
			for z in 0..2 {
				for y in 0..2 {
					for x in 0..2 {
						let w = Vec3::new(
							if x == 0 { 1. - f.x } else { f.x },
							if y == 0 { 1. - f.y } else { f.y },
							if z == 0 { 1. - f.z } else { f.z },
						);
						out += hash(c + IVec3::new(x, y, z), seed) * w.x * w.y * w.z;
					}
				}
			}
			out
		}
		0.7 * value(p * 2.2, seed)
			+ 0.3 * value(p * 4.4 + Vec3::new(19.3, -7.1, 11.7), seed ^ 0x9e3779b9)
	}
	fn triangulate(nodes: &[Vec3], values: &[f32], spacing: Vec3) -> Mesh {
		Self::triangulate_profiled(nodes, values, spacing, None)
	}
	fn triangulate_profiled(
		nodes: &[Vec3],
		values: &[f32],
		spacing: Vec3,
		mut profile: Option<&mut Probe>,
	) -> Mesh {
		let n = (values.len() as f64).cbrt().round() as usize;
		let count = values.len();
		debug_assert_eq!(n * n * n, count);
		SCRATCH.with(|storage| {
			let mut scratch = storage.borrow_mut();
			let Scratch { gradient, positions, normals, edges, vertices, indices } = &mut *scratch;
			gradient.resize(count, Vec3::ZERO);
			positions.clear();
			normals.clear();
			edges.resize(count * 7, None);
			edges.fill(None);
			vertices.resize(count * 7, u32::MAX);
			vertices.fill(u32::MAX);
			indices.clear();
			let gradient_stamp = profile.as_ref().map(|_| std::time::Instant::now());

			for z in 0..n {
				for y in 0..n {
					for x in 0..n {
						let at = x + n * (y + n * z);
						let mut g = Vec3::ZERO;
						for (a, step) in [1, n, n * n].into_iter().enumerate() {
							let q = [x, y, z][a];
							let lo = if q == 0 { at } else { at - step };
							let hi = if q == n - 1 { at } else { at + step };
							g[a] = (values[hi] - values[lo])
								/ (spacing[a] * if q == 0 || q == n - 1 { 1. } else { 2. });
						}
						gradient[at] = -g;
					}
				}
			}
			if let (Some(p), Some(t)) = (profile.as_deref_mut(), gradient_stamp) {
				p.frame.ns[crate::parts::cartoon_flow::diagnostics::Stage::Gradient as usize] +=
					t.elapsed().as_nanos() as u64;
			}
			let tetra_stamp = profile.as_ref().map(|_| std::time::Instant::now());

			for z in 0..n - 1 {
				for y in 0..n - 1 {
					for x in 0..n - 1 {
						let o = x + n * (y + n * z);
						let ids = [
							o,
							o + 1,
							o + n,
							o + n + 1,
							o + n * n,
							o + n * n + 1,
							o + n * n + n,
							o + n * n + n + 1,
						];
						// All six tetrahedra are empty/full when every cube corner has the same sign.
						// This preserves the original strict >0 zero-crossing decision.
						let sign = values[ids[0]] > 0.;
						if ids.iter().all(|&i| (values[i] > 0.) == sign) {
							continue;
						}
						for tet in [
							[0, 1, 3, 7],
							[0, 3, 2, 7],
							[0, 2, 6, 7],
							[0, 6, 4, 7],
							[0, 4, 5, 7],
							[0, 5, 1, 7],
						] {
							let mut inside = [0usize; 4];
							let mut outside = [0usize; 4];
							let (mut ni, mut no) = (0, 0);
							for i in tet {
								let id = ids[i];
								if values[id] > 0. {
									inside[ni] = id;
									ni += 1;
								} else {
									outside[no] = id;
									no += 1;
								}
							}
							if ni == 0 || ni == 4 {
								continue;
							}
							let mut edge = |a: usize, b: usize| {
								let (a, b) = if a < b { (a, b) } else { (b, a) };
								let slot = a * 7
									+ match b - a {
										1 => 0,
										v if v == n => 1,
										v if v == n + 1 => 2,
										v if v == n * n => 3,
										v if v == n * n + 1 => 4,
										v if v == n * n + n => 5,
										_ => 6,
									};
								if let Some(value) = edges[slot] {
									return (value.0, value.1, slot);
								}
								let value = (|| {
									if values[a] == 0. {
										return (nodes[a], gradient[a].normalize_or_zero());
									}
									if values[b] == 0. {
										return (nodes[b], gradient[b].normalize_or_zero());
									}
									let u = (values[a] / (values[a] - values[b])).clamp(0., 1.);
									(
										nodes[a].lerp(nodes[b], u),
										gradient[a].lerp(gradient[b], u).normalize_or_zero(),
									)
								})();
								edges[slot] = Some(value);
								(value.0, value.1, slot)
							};
							let outward =
								outside[..no].iter().map(|i| nodes[*i]).sum::<Vec3>() / no as f32
									- inside[..ni].iter().map(|i| nodes[*i]).sum::<Vec3>()
										/ ni as f32;
							let mut emit = |a: (Vec3, Vec3, usize),
							                mut b: (Vec3, Vec3, usize),
							                mut c: (Vec3, Vec3, usize)| {
								let n = (b.0 - a.0).cross(c.0 - a.0);
								if n.length_squared() == 0. {
									return;
								}
								if n.dot(outward) < 0. {
									std::mem::swap(&mut b, &mut c);
								}
								let face = (b.0 - a.0).cross(c.0 - a.0).normalize_or_zero();
								for (p, mut n, slot) in [a, b, c] {
									let fallback = n.dot(face) <= 0.;
									if n.dot(face) <= 0. {
										n = face;
									}
									let index = if !fallback && vertices[slot] != u32::MAX {
										vertices[slot]
									} else {
										let index = positions.len() as u32;
										positions.push(p.to_array());
										normals.push(n.to_array());
										if !fallback {
											vertices[slot] = index;
										}
										index
									};
									indices.push(index);
								}
							};
							if ni == 1 {
								emit(
									edge(inside[0], outside[0]),
									edge(inside[0], outside[1]),
									edge(inside[0], outside[2]),
								);
							} else if ni == 3 {
								emit(
									edge(outside[0], inside[0]),
									edge(outside[0], inside[1]),
									edge(outside[0], inside[2]),
								);
							} else {
								let a = edge(inside[0], outside[0]);
								let b = edge(inside[0], outside[1]);
								let c = edge(inside[1], outside[0]);
								let d = edge(inside[1], outside[1]);
								emit(a, b, c);
								emit(b, d, c);
							}
						}
					}
				}
			}
			if let (Some(p), Some(t)) = (profile.as_deref_mut(), tetra_stamp) {
				p.frame.ns[crate::parts::cartoon_flow::diagnostics::Stage::Tetra as usize] +=
					t.elapsed().as_nanos() as u64;
			}
			timed!(
				profile,
				Package,
				Mesh::new(
					PrimitiveTopology::TriangleList,
					RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
				)
				.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone())
				.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone())
				.with_inserted_indices(bevy::mesh::Indices::U32(indices.clone()))
			)
		})
	}
	#[cfg(test)]
	fn triangulate_reference(
		nodes: &[Vec3],
		values: &[f32],
		spacing: Vec3,
		mut profile: Option<&mut Probe>,
	) -> Mesh {
		let gradient_stamp = profile.as_ref().map(|_| std::time::Instant::now());
		let mut gradient = vec![Vec3::ZERO; COUNT];
		for z in 0..N {
			for y in 0..N {
				for x in 0..N {
					let at = x + N * (y + N * z);
					let mut g = Vec3::ZERO;
					for (a, step) in [1, N, N * N].into_iter().enumerate() {
						let q = [x, y, z][a];
						let lo = if q == 0 { at } else { at - step };
						let hi = if q == N - 1 { at } else { at + step };
						g[a] = (values[hi] - values[lo])
							/ (spacing[a] * if q == 0 || q == N - 1 { 1. } else { 2. });
					}
					gradient[at] = -g;
				}
			}
		}
		if let (Some(p), Some(t)) = (profile.as_deref_mut(), gradient_stamp) {
			p.frame.ns[crate::parts::cartoon_flow::diagnostics::Stage::Gradient as usize] +=
				t.elapsed().as_nanos() as u64;
		}
		let tetra_stamp = profile.as_ref().map(|_| std::time::Instant::now());
		let mut positions = Vec::<[f32; 3]>::new();
		let mut normals = Vec::<[f32; 3]>::new();
		for z in 0..N - 1 {
			for y in 0..N - 1 {
				for x in 0..N - 1 {
					let o = x + N * (y + N * z);
					let ids = [
						o,
						o + 1,
						o + N,
						o + N + 1,
						o + N * N,
						o + N * N + 1,
						o + N * N + N,
						o + N * N + N + 1,
					];
					for tet in [
						[0, 1, 3, 7],
						[0, 3, 2, 7],
						[0, 2, 6, 7],
						[0, 6, 4, 7],
						[0, 4, 5, 7],
						[0, 5, 1, 7],
					] {
						let mut inside = [0usize; 4];
						let mut outside = [0usize; 4];
						let (mut ni, mut no) = (0, 0);
						for i in tet {
							let id = ids[i];
							if values[id] > 0. {
								inside[ni] = id;
								ni += 1;
							} else {
								outside[no] = id;
								no += 1;
							}
						}
						if ni == 0 || ni == 4 {
							continue;
						}
						let edge = |a: usize, b: usize| {
							let (a, b) = if a < b { (a, b) } else { (b, a) };
							if values[a] == 0. {
								return (nodes[a], gradient[a].normalize_or_zero());
							}
							if values[b] == 0. {
								return (nodes[b], gradient[b].normalize_or_zero());
							}
							let u = (values[a] / (values[a] - values[b])).clamp(0., 1.);
							(
								nodes[a].lerp(nodes[b], u),
								gradient[a].lerp(gradient[b], u).normalize_or_zero(),
							)
						};
						let outward =
							outside[..no].iter().map(|i| nodes[*i]).sum::<Vec3>() / no as f32
								- inside[..ni].iter().map(|i| nodes[*i]).sum::<Vec3>() / ni as f32;
						let mut emit =
							|a: (Vec3, Vec3), mut b: (Vec3, Vec3), mut c: (Vec3, Vec3)| {
								let n = (b.0 - a.0).cross(c.0 - a.0);
								if n.length_squared() == 0. {
									return;
								}
								if n.dot(outward) < 0. {
									std::mem::swap(&mut b, &mut c);
								}
								let face = (b.0 - a.0).cross(c.0 - a.0).normalize_or_zero();
								for (p, mut n) in [a, b, c] {
									if n.dot(face) <= 0. {
										n = face;
									}
									positions.push(p.to_array());
									normals.push(n.to_array());
								}
							};
						if ni == 1 {
							emit(
								edge(inside[0], outside[0]),
								edge(inside[0], outside[1]),
								edge(inside[0], outside[2]),
							);
						} else if ni == 3 {
							emit(
								edge(outside[0], inside[0]),
								edge(outside[0], inside[1]),
								edge(outside[0], inside[2]),
							);
						} else {
							let a = edge(inside[0], outside[0]);
							let b = edge(inside[0], outside[1]);
							let c = edge(inside[1], outside[0]);
							let d = edge(inside[1], outside[1]);
							emit(a, b, c);
							emit(b, d, c);
						}
					}
				}
			}
		}
		if let (Some(p), Some(t)) = (profile.as_deref_mut(), tetra_stamp) {
			p.frame.ns[crate::parts::cartoon_flow::diagnostics::Stage::Tetra as usize] +=
				t.elapsed().as_nanos() as u64;
		}
		timed!(
			profile,
			Package,
			Mesh::new(
				PrimitiveTopology::TriangleList,
				RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
			)
			.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
			.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	#[test]
	fn indexed_expansion_matches_reference_fields() -> Result<(), String> {
		let nodes = (0..COUNT)
			.map(|i| Vec3::new((i % N) as f32, (i / N % N) as f32, (i / (N * N)) as f32) * 0.1)
			.collect::<Vec<_>>();
		for phase in 0..8 {
			let values = nodes
				.iter()
				.enumerate()
				.map(|(i, p)| {
					if (i + phase) % 31 == 0 {
						0.
					} else {
						(0.65 - p.distance(Vec3::new(0.7 + phase as f32 * 0.03, 0.8, 0.9)))
							.min((p.x * 3. + p.z + phase as f32 * 0.2).sin() * 0.2)
					}
				})
				.collect::<Vec<_>>();
			let mesh = ClosedField::triangulate(&nodes, &values, Vec3::splat(0.1));
			let reference =
				ClosedField::triangulate_reference(&nodes, &values, Vec3::splat(0.1), None);
			for attr in [Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_NORMAL] {
				let indexed = mesh
					.attribute(attr.clone())
					.and_then(|v| v.as_float3())
					.ok_or("indexed attribute")?;
				let expanded =
					mesh.indices().ok_or("indices")?.iter().map(|i| indexed[i]).collect::<Vec<_>>();
				assert_eq!(
					expanded.as_slice(),
					reference
						.attribute(attr)
						.and_then(|v| v.as_float3())
						.ok_or("reference attribute")?
				);
			}
		}
		Ok(())
	}

	fn point_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
		let v = b - a;
		a + v * ((p - a).dot(v) / v.length_squared().max(1e-20)).clamp(0., 1.)
	}
	fn point_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
		let ab = b - a;
		let ac = c - a;
		let ap = p - a;
		let d1 = ab.dot(ap);
		let d2 = ac.dot(ap);
		if d1 <= 0. && d2 <= 0. {
			return a;
		}
		let bp = p - b;
		let d3 = ab.dot(bp);
		let d4 = ac.dot(bp);
		if d3 >= 0. && d4 <= d3 {
			return b;
		}
		let vc = d1 * d4 - d3 * d2;
		if vc <= 0. && d1 >= 0. && d3 <= 0. {
			return a + ab * (d1 / (d1 - d3));
		}
		let cp = p - c;
		let d5 = ab.dot(cp);
		let d6 = ac.dot(cp);
		if d6 >= 0. && d5 <= d6 {
			return c;
		}
		let vb = d5 * d2 - d1 * d6;
		if vb <= 0. && d2 >= 0. && d6 <= 0. {
			return a + ac * (d2 / (d2 - d6));
		}
		let va = d3 * d6 - d5 * d4;
		if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
			return b + (c - b) * ((d4 - d3) / (d4 - d3 + d5 - d6));
		}
		a + ab * (vb / (va + vb + vc)) + ac * (vc / (va + vb + vc))
	}
	fn segment_distance(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> f32 {
		let mut best = f32::INFINITY;
		for p in [a, b] {
			best = best.min(p.distance_squared(point_segment(p, c, d)));
		}
		for p in [c, d] {
			best = best.min(p.distance_squared(point_segment(p, a, b)));
		}
		let u = b - a;
		let v = d - c;
		let w = a - c;
		let aa = u.dot(u);
		let bb = u.dot(v);
		let cc = v.dot(v);
		let dd = u.dot(w);
		let ee = v.dot(w);
		let den = aa * cc - bb * bb;
		if den > 1e-15 {
			let s = (bb * ee - cc * dd) / den;
			let t = (aa * ee - bb * dd) / den;
			if (0.0..=1.).contains(&s) && (0.0..=1.).contains(&t) {
				best = best.min((w + s * u - t * v).length_squared());
			}
		}
		best
	}
	fn triangle_distance(a: Vec3, b: Vec3, c: Vec3, p: Vec3, q: Vec3) -> f32 {
		let mut best = p
			.distance_squared(point_triangle(p, a, b, c))
			.min(q.distance_squared(point_triangle(q, a, b, c)));
		for (x, y) in [(a, b), (b, c), (c, a)] {
			best = best.min(segment_distance(p, q, x, y));
		}
		let n = (b - a).cross(c - a);
		let den = n.dot(q - p);
		if den.abs() > 1e-12 {
			let t = n.dot(a - p) / den;
			if (0.0..=1.).contains(&t) {
				let x = p + t * (q - p);
				if x.distance_squared(point_triangle(x, a, b, c)) < 1e-12 {
					return 0.;
				}
			}
		}
		best.sqrt()
	}
	#[test]
	fn reconstructed_contact_is_closed_oriented_and_triangle_separated() -> Result<(), String> {
		let mut nodes = Vec::new();
		let mut values = Vec::new();
		let lo = Vec3::splat(-1.2);
		let hi = Vec3::splat(1.2);
		let p = Vec3::new(0.31, -0.4, 0.09);
		let q = p + Vec3::Y * 0.8;
		for z in 0..N {
			for y in 0..N {
				for x in 0..N {
					let v =
						lo + (hi - lo) * Vec3::new(x as f32, y as f32, z as f32) / (N - 1) as f32;
					nodes.push(v);
					values.push(ClosedField::intersect(
						0.91 - v.length(),
						v.distance(point_segment(v, p, q)) - 0.315,
						0.08,
					));
				}
			}
		}
		let mesh = ClosedField::triangulate(&nodes, &values, (hi - lo) / (N - 1) as f32);
		let positions = mesh
			.attribute(Mesh::ATTRIBUTE_POSITION)
			.and_then(|p| p.as_float3())
			.ok_or("positions")?;
		let normals = mesh
			.attribute(Mesh::ATTRIBUTE_NORMAL)
			.and_then(|p| p.as_float3())
			.ok_or("normals")?;
		let mut probe = Probe::default();
		let profiled = ClosedField::triangulate_profiled(
			&nodes,
			&values,
			(hi - lo) / (N - 1) as f32,
			Some(&mut probe),
		);
		assert_eq!(
			positions,
			profiled
				.attribute(Mesh::ATTRIBUTE_POSITION)
				.and_then(|p| p.as_float3())
				.ok_or("profiled positions")?
		);
		assert_eq!(
			normals,
			profiled
				.attribute(Mesh::ATTRIBUTE_NORMAL)
				.and_then(|p| p.as_float3())
				.ok_or("profiled normals")?
		);
		let reference =
			ClosedField::triangulate_reference(&nodes, &values, (hi - lo) / (N - 1) as f32, None);
		let indices = mesh.indices().ok_or("indices")?;
		let positions = indices.iter().map(|i| positions[i]).collect::<Vec<_>>();
		let normals = indices.iter().map(|i| normals[i]).collect::<Vec<_>>();
		assert_eq!(
			positions.as_slice(),
			reference
				.attribute(Mesh::ATTRIBUTE_POSITION)
				.and_then(|v| v.as_float3())
				.ok_or("reference positions")?
		);
		assert_eq!(
			normals.as_slice(),
			reference
				.attribute(Mesh::ATTRIBUTE_NORMAL)
				.and_then(|v| v.as_float3())
				.ok_or("reference normals")?
		);
		assert!(!positions.is_empty());
		let mut weld = HashMap::new();
		let mut edges = HashMap::<(usize, usize), (u32, i32)>::new();
		let mut minimum = f32::INFINITY;
		for (i, tri) in positions.chunks_exact(3).enumerate() {
			let v: [Vec3; 3] = std::array::from_fn(|j| Vec3::from_array(tri[j]));
			let n = (v[1] - v[0]).cross(v[2] - v[0]);
			assert!(n.length_squared() > 1e-18);
			let normal =
				normals[i * 3..i * 3 + 3].iter().map(|p| Vec3::from_array(*p)).sum::<Vec3>();
			assert!(n.dot(normal) > 0.);
			minimum = minimum.min(triangle_distance(v[0], v[1], v[2], p, q));
			let ids = v.map(|p| {
				let key = p.to_array().map(|x| (x * 1e5).round() as i32);
				let next = weld.len();
				*weld.entry(key).or_insert(next)
			});
			for (a, b) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
				assert_ne!(a, b);
				let e = edges.entry((a.min(b), a.max(b))).or_default();
				e.0 += 1;
				e.1 += if a < b { 1 } else { -1 };
			}
		}
		assert!(
			edges.values().all(|v| *v == (2, 0)),
			"boundary={}, nonmanifold={}, orientation={}",
			edges.values().filter(|v| v.0 == 1).count(),
			edges.values().filter(|v| v.0 > 2).count(),
			edges.values().filter(|v| v.0 == 2 && v.1 != 0).count()
		);
		assert!(minimum >= 0.28 - 0.002, "triangle clearance {minimum}");
		Ok(())
	}
}
