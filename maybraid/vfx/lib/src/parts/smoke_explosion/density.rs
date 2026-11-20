//! Portable Combined appearance, shared CPU/GPU primitive preparation.
use crate::parts::{
	packaged::{PackagedAssetError as Error, StaticVolume},
	smoke_explosion::track::{ExplosionTrack, Frame, State},
};
use bevy::{
	asset::{io::Reader, AssetLoader, LoadContext},
	prelude::*,
};
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
pub struct Appearance {
	pub id: usize,
	pub seed: u32,
	pub noise_o: [f32; 3],
	pub noise_k: [f32; 3],
	#[serde(rename = "shrink_T")]
	pub shrink_t: Option<f32>,
	pub swap_gain: Option<Swap>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Swap {
	pub g0: f32,
	#[serde(rename = "T")]
	pub duration: f32,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Release {
	pub t: f32,
	#[serde(rename = "T")]
	pub position: [f32; 3],
	pub q: [f32; 4],
	pub s: [f32; 3],
	pub v: [f32; 3],
}
#[derive(Clone, Debug, Deserialize)]
pub struct Patch {
	pub pid: usize,
	pub parent: usize,
	pub anchor: [f32; 3],
	pub normal: [f32; 3],
	pub frame: [[f32; 3]; 3],
	pub radii: [f32; 3],
	pub drift: [f32; 3],
	pub noise_seed: u32,
	pub release: Option<Release>,
}
#[derive(Asset, TypePath, Clone, Debug)]
pub struct CombinedExplosionAsset {
	#[dependency]
	pub track: Handle<ExplosionTrack>,
	#[dependency]
	pub fields: Vec<Handle<StaticVolume>>,
	pub mesh_fields: Vec<Option<usize>>,
	pub appearances: Vec<Appearance>,
	pub patches: Vec<Patch>,
	pub grains: Vec<GrainBirth>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct GrainBirth {
	pub id: usize,
	pub birth: f32,
	pub world_origin: [f32; 3],
	pub initial_velocity: [f32; 3],
	pub radius: f32,
	pub drag: f32,
	#[serde(rename = "shrink_T")]
	pub shrink_t: f32,
	pub shade: f32,
	pub orient_seed: u32,
}
#[derive(Default, TypePath)]
pub struct CombinedLoader;
impl AssetLoader for CombinedLoader {
	type Asset = CombinedExplosionAsset;
	type Settings = ();
	type Error = Error;
	async fn load(
		&self,
		r: &mut dyn Reader,
		_: &(),
		ctx: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Error> {
		let mut bytes = Vec::new();
		r.read_to_end(&mut bytes).await?;
		if crate::parts::smoke_explosion::sha256::digest(&bytes)
			!= "2c2f2b8862b8b7d5c780658a1ec4f0fc38ce7b9c2e071af22af688fb8d4da9de"
		{
			return Err(Error::invalid("unsupported Combined appearance contract: update evaluator and parity tests together"));
		}
		let v: serde_json::Value = serde_json::from_slice(&bytes)?;
		if v["schema"] != "maybraid.combined_explosion.combined.v1"
			|| v["cartoon"]["track_sha256"]
				!= "1dd6f7bd1b76061eab99613616b71198d53d9f474ecc16ff1040ef89aefe914c"
		{
			return Err(Error::invalid("incompatible Combined explosion manifest"));
		}
		let source = ctx.path().clone();
		let path = |s: &str| source.resolve_embed_str(s).map_err(|e| Error::invalid(e.to_string()));
		let appearances: Vec<Appearance> = serde_json::from_value(v["elements"].clone())?;
		let patches: Vec<Patch> = serde_json::from_value(v["fine"]["patches"].clone())?;
		if appearances.len() != 349
			|| appearances.iter().enumerate().any(|(i, a)| {
				a.id != i
					|| !Vec3::from_array(a.noise_o).is_finite()
					|| !Vec3::from_array(a.noise_k).is_finite()
			}) || patches.len() != 51
			|| patches.iter().enumerate().any(|(i, p)| {
				p.pid != i || p.parent >= 349 || Vec3::from_array(p.radii).min_element() <= 0.
			}) {
			return Err(Error::invalid("invalid Combined appearance tables"));
		}
		let mut fields = Vec::new();
		let mut mesh_fields = Vec::new();
		for f in v["fields_by_mesh_index"].as_array().ok_or_else(|| Error::invalid("fields"))? {
			if let Some(f) = f.as_str() {
				mesh_fields.push(Some(fields.len()));
				fields.push(ctx.load(path(f)?));
			} else {
				mesh_fields.push(None);
			}
		}
		if fields.len() != 16 || mesh_fields.len() != 17 {
			return Err(Error::invalid("expected sixteen density fields"));
		}
		let grain_bytes = ctx.read_asset_bytes(path("combined_explosion.grains.json")?).await?;
		if crate::parts::smoke_explosion::sha256::digest(&grain_bytes)
			!= "d81b5e326282ef5821715d32881eaec39e8dc126922793d68ff6a431924bbe1d"
		{
			return Err(Error::invalid("unsupported grain integrator contract"));
		}
		let grain: serde_json::Value = serde_json::from_slice(&grain_bytes)?;
		if grain["schema"] != "maybraid.combined_explosion.grains.v1" {
			return Err(Error::invalid("grain schema"));
		}
		let grains: Vec<GrainBirth> = serde_json::from_value(grain["births"].clone())?;
		if grains.len() != 96
			|| grains.iter().map(|g| g.id).collect::<std::collections::HashSet<_>>().len() != 96
			|| grains.iter().any(|g| {
				g.drag <= 0. || g.radius <= 0. || !Vec3::from_array(g.initial_velocity).is_finite()
			}) {
			return Err(Error::invalid("grain table"));
		}
		Ok(CombinedExplosionAsset {
			track: ctx.load(path("../cartoon_explosion/cartoon_explosion.explosion.json")?),
			fields,
			mesh_fields,
			appearances,
			patches,
			grains,
		})
	}
	fn extensions(&self) -> &[&str] {
		&["combined_explosion.combined.json"]
	}
}
pub fn smooth(x: f32) -> f32 {
	let x = x.clamp(0., 1.);
	x * x * (3. - 2. * x)
}
pub fn noise(p: Vec3, seed: u32) -> f32 {
	let phi = (seed & 65535) as f32 * 0.000219;
	0.5 + 0.22 * (p.dot(Vec3::new(2.1, 1.7, 0.9)) + phi).sin()
		+ 0.16 * (p.dot(Vec3::new(-1.4, 3.2, 2.6)) + 1.71 * phi).sin()
		+ 0.10 * (p.dot(Vec3::new(4.7, -0.8, 3.1)) + 0.63 * phi).sin()
}
#[derive(Clone, Debug)]
pub struct Primitive {
	pub inverse: Mat4,
	pub noise_o: Vec3,
	pub noise_k: Vec3,
	pub parameters: Vec4,
	pub extra: Vec4,
	pub min: Vec3,
	pub max: Vec3,
}
impl Primitive {
	pub fn rows(&self) -> [Vec4; 10] {
		[
			self.inverse.row(0),
			self.inverse.row(1),
			self.inverse.row(2),
			self.noise_o.extend(0.),
			self.noise_k.extend(0.),
			self.parameters,
			self.extra,
			self.min.extend(0.),
			self.max.extend(0.),
			Vec4::new(0., 0., 0., self.parameters.w),
		]
	}
	pub fn bounds(model: Mat4, lo: Vec3, hi: Vec3) -> (Vec3, Vec3) {
		let c = model.transform_point3((lo + hi) * 0.5);
		let h = (hi - lo) * 0.5;
		let e = model.x_axis.xyz().abs() * h.x
			+ model.y_axis.xyz().abs() * h.y
			+ model.z_axis.xyz().abs() * h.z;
		(c - e, c + e)
	}
	pub fn density(&self, x: Vec3, rest: impl Fn(usize, Vec3) -> f32) -> f32 {
		if x.cmplt(self.min).any() || x.cmpgt(self.max).any() {
			return 0.;
		}
		let r = self.inverse.transform_point3(x);
		let kind = self.parameters.w as u32;
		if kind == 0 {
			smooth(rest(self.parameters.x as usize, r) / 0.18)
				* (1.
					+ 0.32
						* (2. * noise(self.noise_o + self.noise_k * r, self.parameters.y as u32)
							- 1.)) * self.parameters.z
		} else {
			let y = r.y;
			if !(0.0..=1.).contains(&y) {
				return 0.;
			}
			let hw = 0.03 * (1. - y).powf(0.9);
			if hw <= 1e-8 {
				return 0.;
			}
			let rn = Vec2::new(r.x / hw, r.z / (0.75 * hw)).length();
			7. * smooth(1. - rn) * smooth((1. - self.extra.x - y) / 0.08)
		}
	}
}
impl CombinedExplosionAsset {
	pub fn main_primitives(
		&self,
		track: &ExplosionTrack,
		frame: &Frame,
		t: f32,
		volumes: &Assets<StaticVolume>,
	) -> Vec<Primitive> {
		let mut out = Vec::with_capacity(frame.states.len());
		for s in &frame.states {
			if !s.visible() {
				continue;
			}
			let e = &track.manifest.elements[s.id];
			let model = s.model();
			let inv = model.inverse();
			if let Some(field) = self.mesh_fields[e.mesh] {
				let Some(v) = volumes.get(&self.fields[field]) else {
					continue;
				};
				let a = &self.appearances[s.id];
				let mut concentration = 1. - 0.6 * smooth((t - 5.5) / 7.5);
				if let Some(duration) = a.shrink_t {
					let end = (e.first_sample + e.sample_count - 1) as f32 / 30.;
					concentration *= 1. - 0.5 * smooth((t - (end - duration)) / duration);
				}
				if let Some(g) = &a.swap_gain {
					concentration *= 1. + (g.g0 - 1.) * (1. - smooth((t - e.birth_s) / g.duration));
				}
				let (min, max) = Primitive::bounds(
					model,
					Vec3::from_array(v.manifest.bounds.min),
					Vec3::from_array(v.manifest.bounds.max),
				);
				out.push(Primitive {
					inverse: inv,
					noise_o: Vec3::from_array(a.noise_o),
					noise_k: Vec3::from_array(a.noise_k),
					parameters: Vec4::new(field as f32, (a.seed & 65535) as f32, concentration, 0.),
					extra: Vec4::ZERO,
					min,
					max,
				});
			} else {
				let (min, max) = Primitive::bounds(
					model,
					Vec3::new(-0.03, 0., -0.03),
					Vec3::new(0.03, 1., 0.03),
				);
				out.push(Primitive {
					inverse: inv,
					noise_o: Vec3::ZERO,
					noise_k: Vec3::ZERO,
					parameters: Vec4::new(0., 0., 0., 2.),
					extra: Vec4::new(s.dissolve, 0., 0., 0.),
					min,
					max,
				});
			}
		}
		out
	}
	pub fn fine_states(&self, track: &ExplosionTrack, t: f32) -> Vec<Fine> {
		let mut out = Vec::new();
		for p in &self.patches {
			let e = &track.manifest.elements[p.parent];
			let mut at = t;
			let mut fade = 1.;
			let mut growth = 1.;
			let mut drift = Vec3::ZERO;
			let s = if let Some(r) = &p.release {
				if t >= r.t {
					let tau = t - r.t;
					if tau >= 1.5 {
						continue;
					}
					at = r.t;
					fade = 1. - smooth(tau / 1.5);
					growth = 1. + 0.2 * smooth(tau / 1.5);
					drift = (Vec3::from_array(r.v) + Vec3::Y * 0.15) * tau;
					State {
						id: p.parent,
						position: Vec3::from_array(r.position),
						rotation: Quat::from_array(r.q).normalize(),
						scale: Vec3::from_array(r.s),
						dissolve: 0.,
						tone: 0.,
						open_dir: Vec3::ZERO,
						openness: 1.,
					}
				} else {
					let Some(s) = track.sample(p.parent, t) else {
						continue;
					};
					s
				}
			} else {
				let Some(s) = track.sample(p.parent, t) else {
					continue;
				};
				s
			};
			if !s.visible() {
				continue;
			}
			let prog = smooth((at - e.birth_s) / 1.8);
			let anchor = Vec3::from_array(p.anchor);
			let center = anchor
				+ Vec3::from_array(p.normal) * (0.035 + 0.09 * prog)
				+ Vec3::from_array(p.drift) * prog;
			let matrix = Mat3::from_cols(
				Vec3::from_array(p.frame[0]),
				Vec3::from_array(p.frame[1]),
				Vec3::from_array(p.frame[2]),
			)
			.transpose();
			let radii = Vec3::from_array(p.radii) * (0.72 + 0.28 * prog) * growth;
			let model = s.model();
			let mut moved = model;
			moved.w_axis += drift.extend(0.);
			let shape = moved
				* Mat4::from_translation(center)
				* Mat4::from_mat3(matrix)
				* Mat4::from_scale(radii);
			let (min, max) = Primitive::bounds(shape, -Vec3::ONE, Vec3::ONE);
			out.push(Fine {
				inverse: moved.inverse(),
				frame: matrix,
				radii,
				center,
				anchor,
				seed: p.noise_seed,
				amplitude: 0.22 * smooth((t - e.birth_s.max(0.3)) / 0.6) * fade,
				min,
				max,
			});
		}
		out
	}
}
#[derive(Clone, Debug)]
pub struct Fine {
	pub inverse: Mat4,
	pub frame: Mat3,
	pub radii: Vec3,
	pub center: Vec3,
	pub anchor: Vec3,
	pub seed: u32,
	pub amplitude: f32,
	pub min: Vec3,
	pub max: Vec3,
}
impl Fine {
	pub fn density(&self, x: Vec3) -> f32 {
		let r = self.inverse.transform_point3(x);
		let q = self.frame.transpose() * (r - self.center) / self.radii;
		self.amplitude
			* (1. - q.length_squared()).max(0.).powi(2)
			* (0.35 + 0.65 * noise((r - self.center + self.anchor) * 2.8, self.seed)).powi(2)
			* smooth(x.y.max(0.) / 0.035)
	}
	pub fn rows(&self) -> [Vec4; 10] {
		let q = Mat4::from_scale(self.radii.recip())
			* Mat4::from_mat3(self.frame.transpose())
			* Mat4::from_translation(-self.center)
			* self.inverse;
		[
			q.row(0),
			q.row(1),
			q.row(2),
			self.inverse.row(0),
			self.inverse.row(1),
			self.inverse.row(2),
			self.center.extend(self.amplitude),
			self.anchor.extend((self.seed & 65535) as f32),
			self.min.extend(1.),
			self.max.extend(1.),
		]
	}
}
impl CombinedExplosionAsset {
	/// This export is axis-aligned, voxel-centred, normalized around each exact mesh pivot.
	pub fn validate_fields(
		&self,
		track: &ExplosionTrack,
		volumes: &Assets<StaticVolume>,
	) -> Result<(), Error> {
		for (i, e) in track.manifest.meshes.iter().enumerate() {
			let expected = if i < 10 {
				format!("puff_{i}")
			} else if i == 10 {
				"spike_0".into()
			} else {
				format!("puff_small_{}", i - 11)
			};
			if e.name != expected {
				return Err(Error::invalid(
					"explosion mesh table does not match density variant order",
				));
			}
			let Some(field) = self.mesh_fields[i] else {
				if i != 10 {
					return Err(Error::invalid("missing puff density"));
				}
				continue;
			};
			let v = volumes
				.get(&self.fields[field])
				.ok_or_else(|| Error::invalid("density dependency unavailable"))?;
			let m = &v.manifest;
			let n = if i < 10 { 64 } else { 32 };
			let lo = Vec3::from_array(m.bounds.min);
			let hi = Vec3::from_array(m.bounds.max);
			let step = (hi - lo) / n as f32;
			let expected =
				Mat4::from_scale_rotation_translation(step, Quat::IDENTITY, lo + step * 0.5);
			if m.dimensions != [n; 3]
				|| m.density_scale != 1.
				|| m.index_to_local_matrix()
					.to_cols_array()
					.iter()
					.zip(expected.to_cols_array())
					.any(|(a, b)| (a - b).abs() > 1e-6)
			{
				return Err(Error::invalid(
					"unsupported density dimensions or voxel-centre transform",
				));
			}
		}
		Ok(())
	}
}
