//! Temporary mesh BVH used once to build shared signed-distance lattices.
use bevy::prelude::*;
#[derive(Clone, Copy)]
struct Triangle([Vec3; 3]);
impl Triangle {
	fn closest(self, p: Vec3) -> Vec3 {
		let [a, b, c] = self.0;
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
	fn ray(self, p: Vec3, d: Vec3) -> Option<f32> {
		let [a, b, c] = self.0;
		let e = b - a;
		let f = c - a;
		let h = d.cross(f);
		let det = e.dot(h);
		if det.abs() < 1e-9 {
			return None;
		}
		let inverse = 1. / det;
		let s = p - a;
		let u = s.dot(h) * inverse;
		if !(0.0..=1.).contains(&u) {
			return None;
		}
		let q = s.cross(e);
		let v = d.dot(q) * inverse;
		if v < 0. || u + v > 1. {
			return None;
		}
		let t = f.dot(q) * inverse;
		(t > 1e-6).then_some(t)
	}
}
struct Node {
	lo: Vec3,
	hi: Vec3,
	start: usize,
	count: usize,
	children: Option<(usize, usize)>,
}
pub(crate) struct DistanceMesh {
	triangles: Vec<Triangle>,
	nodes: Vec<Node>,
}
impl DistanceMesh {
	pub fn from_mesh(mesh: &Mesh) -> Option<Self> {
		let points = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
		let indices = mesh
			.indices()
			.map(|v| v.iter().collect::<Vec<_>>())
			.unwrap_or_else(|| (0..points.len()).collect());
		if indices.len() % 3 != 0 || indices.is_empty() {
			return None;
		}
		let triangles = indices
			.chunks_exact(3)
			.map(|t| {
				Some(Triangle([
					Vec3::from_array(*points.get(t[0])?),
					Vec3::from_array(*points.get(t[1])?),
					Vec3::from_array(*points.get(t[2])?),
				]))
			})
			.collect::<Option<Vec<_>>>()?;
		let mut s = Self { triangles, nodes: Vec::new() };
		Self::build(&mut s.triangles, 0, &mut s.nodes);
		Some(s)
	}
	fn build(triangles: &mut [Triangle], start: usize, nodes: &mut Vec<Node>) -> usize {
		let lo = triangles.iter().flat_map(|t| t.0).fold(Vec3::splat(f32::INFINITY), Vec3::min);
		let hi = triangles
			.iter()
			.flat_map(|t| t.0)
			.fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
		let id = nodes.len();
		nodes.push(Node { lo, hi, start, count: triangles.len(), children: None });
		if triangles.len() > 8 {
			let e = hi - lo;
			let axis = if e.x >= e.y && e.x >= e.z {
				0
			} else if e.y >= e.z {
				1
			} else {
				2
			};
			triangles.sort_by(|a, b| {
				a.0.iter()
					.map(|p| p[axis])
					.sum::<f32>()
					.total_cmp(&b.0.iter().map(|p| p[axis]).sum::<f32>())
			});
			let mid = triangles.len() / 2;
			let (a, b) = triangles.split_at_mut(mid);
			let left = Self::build(a, start, nodes);
			let right = Self::build(b, start + mid, nodes);
			nodes[id].children = Some((left, right));
		}
		id
	}
	pub fn bounds(&self) -> (Vec3, Vec3) {
		(self.nodes[0].lo, self.nodes[0].hi)
	}
	fn box_distance(node: &Node, p: Vec3) -> f32 {
		((node.lo - p).max(Vec3::ZERO) + (p - node.hi).max(Vec3::ZERO)).length_squared()
	}
	fn nearest(&self, id: usize, p: Vec3, best: &mut f32) {
		let n = &self.nodes[id];
		if Self::box_distance(n, p) > *best {
			return;
		}
		if let Some((a, b)) = n.children {
			let (near, far) =
				if Self::box_distance(&self.nodes[a], p) < Self::box_distance(&self.nodes[b], p) {
					(a, b)
				} else {
					(b, a)
				};
			self.nearest(near, p, best);
			self.nearest(far, p, best);
		} else {
			for t in &self.triangles[n.start..n.start + n.count] {
				*best = best.min(p.distance_squared(t.closest(p)));
			}
		}
	}
	fn hits(&self, id: usize, p: Vec3, d: Vec3, hits: &mut Vec<f32>) {
		let n = &self.nodes[id];
		let a = (n.lo - p) / d;
		let b = (n.hi - p) / d;
		if a.min(b).max_element().max(0.) > a.max(b).min_element() {
			return;
		}
		if let Some((a, b)) = n.children {
			self.hits(a, p, d, hits);
			self.hits(b, p, d, hits);
		} else {
			for t in &self.triangles[n.start..n.start + n.count] {
				if let Some(v) = t.ray(p, d) {
					hits.push(v);
				}
			}
		}
	}
	/// Positive inside, negative outside. The source must have closed input topology.
	pub fn sample(&self, p: Vec3) -> f32 {
		let mut distance = f32::INFINITY;
		self.nearest(0, p, &mut distance);
		let (lo, hi) = self.bounds();
		if p.cmplt(lo).any() || p.cmpgt(hi).any() {
			return -distance.sqrt();
		}
		let mut hits = Vec::with_capacity(8);
		self.hits(0, p, Vec3::new(1., 0.37139067, 0.217583).normalize(), &mut hits);
		hits.sort_by(f32::total_cmp);
		hits.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
		if hits.len() % 2 == 1 {
			distance.sqrt()
		} else {
			-distance.sqrt()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn cube_distance_and_sign() -> Result<(), String> {
		let mesh = Mesh::from(Cuboid::new(2., 2., 2.));
		let field = DistanceMesh::from_mesh(&mesh).ok_or("mesh")?;
		assert!((field.sample(Vec3::ZERO) - 1.).abs() < 1e-5);
		assert!((field.sample(Vec3::new(1.5, 0., 0.)) + 0.5).abs() < 1e-5);
		assert!((field.sample(Vec3::new(0.9, 0.7, 0.8)) - 0.1).abs() < 1e-5);
		Ok(())
	}
	#[test]
	fn analytic_deformation_normal_agrees_with_finite_jacobian() {
		use crate::parts::cartoon_wave::motion::Deformation;
		for clip in 0..3 {
			for phase in 0..13 {
				let d = Deformation::at(phase as f32 * 0.25, clip);
				for i in 0..21 {
					let p = Vec3::new(
						-0.9 + i as f32 * 0.1,
						0.1 + i as f32 * 0.06,
						(i as f32 * 0.7).sin() * 0.8,
					);
					let n = Vec3::new(0.3, 0.7, -0.2).normalize();
					assert!(d.normal(p, n).distance(d.normal_analytic(p, n)) < 0.003);
				}
			}
		}
	}
}
