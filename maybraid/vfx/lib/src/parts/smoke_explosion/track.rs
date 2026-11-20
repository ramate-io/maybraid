//! Rendering-independent sparse authored track. No authoring dependencies at runtime.
use crate::parts::packaged::PackagedAssetError as Error;
use bevy::{
	asset::{io::Reader, AssetLoader, LoadContext},
	prelude::*,
};
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
pub struct Element {
	pub id: usize,
	pub kind: String,
	pub mesh: usize,
	pub lobe: Option<usize>,
	pub lod_rank: Option<usize>,
	pub lod_scale: LodScale,
	pub parent: Option<usize>,
	pub first_sample: usize,
	pub sample_count: usize,
	pub byte_offset: usize,
	pub birth_s: f32,
}
#[derive(Clone, Debug, Deserialize)]
pub struct LodScale {
	pub medium: f32,
	pub low: f32,
}
#[derive(Clone, Debug, Deserialize)]
pub struct MeshInfo {
	pub index: usize,
	pub name: String,
	pub file: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct TrackInfo {
	pub file: String,
	pub floats_per_sample: usize,
	pub byte_length: usize,
	pub sha256: String,
	pub fields: Vec<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
	pub schema: String,
	pub duration_s: f32,
	pub sample_rate_hz: f32,
	pub sample_count: usize,
	pub track: TrackInfo,
	pub meshes: Vec<MeshInfo>,
	pub elements: Vec<Element>,
	pub lobe_centroids: Vec<Vec<[f32; 3]>>,
	pub mass_height: Vec<f32>,
}
#[derive(Clone, Copy, Debug)]
pub struct State {
	pub id: usize,
	pub position: Vec3,
	pub rotation: Quat,
	pub scale: Vec3,
	pub dissolve: f32,
	pub tone: f32,
	pub open_dir: Vec3,
	pub openness: f32,
}
impl State {
	pub fn model(self) -> Mat4 {
		Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
	}
	pub fn visible(self) -> bool {
		self.scale.min_element() > 1e-7
	}
}
#[derive(Default, Clone, Debug)]
pub struct Frame {
	pub states: Vec<State>,
	pub centroids: Vec<Vec3>,
	pub mass_height: f32,
}
#[derive(Asset, TypePath, Clone, Debug)]
pub struct ExplosionTrack {
	pub manifest: Manifest,
	pub samples: Vec<[f32; 16]>,
	#[dependency]
	pub meshes: Vec<Handle<Mesh>>,
}
impl ExplosionTrack {
	pub fn decode(manifest: Manifest, bytes: &[u8]) -> Result<Self, Error> {
		let fail = || Error::invalid("invalid authored explosion track");
		let m = &manifest;
		if m.schema != "maybraid.cartoon-explosion.v1"
			|| m.duration_s != 14.
			|| m.sample_rate_hz != 30.
			|| m.sample_count != 421
			|| m.elements.len() != 349
			|| m.meshes.len() != 17
			|| m.track.floats_per_sample != 16
			|| bytes.len() != m.track.byte_length
			|| bytes.len() != 2_970_304
			|| m.track.fields.join(",")
				!= "px,py,pz,qx,qy,qz,qw,sx,sy,sz,dissolve,tone,ox,oy,oz,openness"
		{
			return Err(fail());
		}
		if m.track.sha256 != "1dd6f7bd1b76061eab99613616b71198d53d9f474ecc16ff1040ef89aefe914c"
			|| crate::parts::smoke_explosion::sha256::digest(bytes) != m.track.sha256
		{
			return Err(Error::invalid("explosion track SHA-256 mismatch"));
		}
		if m.lobe_centroids.len() != 421
			|| m.mass_height.len() != 421
			|| m.mass_height.iter().any(|x| !x.is_finite() || *x < 1.)
			|| m.lobe_centroids
				.iter()
				.any(|c| c.len() != 11 || c.iter().any(|p| !Vec3::from_array(*p).is_finite()))
		{
			return Err(fail());
		}
		let mut offset = 0usize;
		for (i, e) in m.elements.iter().enumerate() {
			if e.id != i
				|| e.mesh >= 17
				|| e.lobe.is_some_and(|l| l >= 11)
				|| e.parent.is_some_and(|p| p >= i)
				|| e.sample_count < 2
				|| e.first_sample.checked_add(e.sample_count).is_none_or(|v| v > 421)
				|| e.byte_offset != offset
				|| !e.birth_s.is_finite()
				|| !e.lod_scale.medium.is_finite()
				|| !e.lod_scale.low.is_finite()
			{
				return Err(fail());
			}
			offset = offset
				.checked_add(e.sample_count.checked_mul(64).ok_or_else(fail)?)
				.ok_or_else(fail)?;
			if offset > bytes.len() {
				return Err(fail());
			}
		}
		if offset != bytes.len() || m.meshes.iter().enumerate().any(|(i, m)| m.index != i) {
			return Err(fail());
		}
		let mut samples = Vec::with_capacity(bytes.len() / 64);
		for sample in bytes.chunks_exact(64) {
			let mut v = [0.; 16];
			for (i, b) in sample.chunks_exact(4).enumerate() {
				v[i] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
			}
			let q = Quat::from_array([v[3], v[4], v[5], v[6]]);
			if v.iter().any(|x| !x.is_finite())
				|| (q.length() - 1.).abs() > 1e-3
				|| v[7..10].iter().any(|s| *s < 0.)
			{
				return Err(fail());
			}
			samples.push(v);
		}
		for e in &manifest.elements {
			for pair in samples[e.byte_offset / 64..e.byte_offset / 64 + e.sample_count].windows(2)
			{
				let q = |v: &[f32; 16]| Quat::from_array([v[3], v[4], v[5], v[6]]);
				if q(&pair[0]).dot(q(&pair[1])) < -1e-5 {
					return Err(Error::invalid("quaternion hemisphere discontinuity"));
				}
			}
		}
		Ok(Self { manifest, samples, meshes: Vec::new() })
	}
	pub fn sample(&self, id: usize, t: f32) -> Option<State> {
		let e = self.manifest.elements.get(id)?;
		if !t.is_finite() {
			return None;
		}
		let t = t.clamp(0., self.manifest.duration_s);
		let rate = self.manifest.sample_rate_hz;
		let start = e.first_sample as f32 / rate;
		let end = (e.first_sample + e.sample_count - 1) as f32 / rate;
		if t < start || t > end {
			return None;
		}
		// Compare in time units: (n/30)*30 may round beyond n in float32.
		// Canonical endpoints must retain their exact pad values and existence.
		let local = if t == start {
			0.
		} else if t == end {
			(e.sample_count - 1) as f32
		} else {
			(t * rate - e.first_sample as f32).clamp(0., (e.sample_count - 1) as f32)
		};

		let lo = (local.floor() as usize).min(e.sample_count - 1);
		let hi = (lo + 1).min(e.sample_count - 1);
		let a = self.samples[e.byte_offset / 64 + lo];
		let b = self.samples[e.byte_offset / 64 + hi];
		let f = local - lo as f32;
		let v = std::array::from_fn::<_, 16, _>(|i| a[i] + (b[i] - a[i]) * f);
		let qa = Quat::from_array([a[3], a[4], a[5], a[6]]);
		let qb = Quat::from_array([b[3], b[4], b[5], b[6]]);
		Some(State {
			id,
			position: Vec3::new(v[0], v[1], v[2]),
			rotation: qa.lerp(qb, f).normalize(),
			scale: Vec3::new(v[7], v[8], v[9]),
			dissolve: v[10],
			tone: v[11],
			open_dir: Vec3::new(v[12], v[13], v[14]),
			openness: v[15],
		})
	}
	pub fn evaluate(&self, t: f32, out: &mut Frame) {
		out.states.clear();
		for id in 0..self.manifest.elements.len() {
			if let Some(s) = self.sample(id, t) {
				out.states.push(s);
			}
		}
		let u = (t * self.manifest.sample_rate_hz).clamp(0., 420.);
		let a = u.floor() as usize;
		let b = (a + 1).min(420);
		let f = u - a as f32;
		out.mass_height =
			self.manifest.mass_height[a] * (1. - f) + self.manifest.mass_height[b] * f;
		out.centroids.clear();
		out.centroids.extend((0..11).map(|i| {
			Vec3::from_array(self.manifest.lobe_centroids[a][i])
				.lerp(Vec3::from_array(self.manifest.lobe_centroids[b][i]), f)
		}));
	}
}
#[derive(Default, TypePath)]
pub struct TrackLoader;
impl AssetLoader for TrackLoader {
	type Asset = ExplosionTrack;
	type Settings = ();
	type Error = Error;
	async fn load(
		&self,
		reader: &mut dyn Reader,
		_: &(),
		ctx: &mut LoadContext<'_>,
	) -> Result<ExplosionTrack, Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let m: Manifest = serde_json::from_slice(&bytes)?;
		let path = ctx.path().clone();
		let track = path
			.resolve_embed_str(&m.track.file)
			.map_err(|e| Error::invalid(e.to_string()))?;
		let bytes = ctx.read_asset_bytes(track).await?;
		let mut asset = ExplosionTrack::decode(m, &bytes)?;
		for mesh in &asset.manifest.meshes {
			asset.meshes.push(
				ctx.load(
					path.resolve_embed_str(&format!("{}#Mesh0/Primitive0", mesh.file))
						.map_err(|e| Error::invalid(e.to_string()))?,
				),
			);
		}
		Ok(asset)
	}
	fn extensions(&self) -> &[&str] {
		&["explosion.json"]
	}
}
