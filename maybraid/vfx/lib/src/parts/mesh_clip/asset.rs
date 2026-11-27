use crate::parts::packaged::PackagedAssetError;
use bevy::{
	asset::{io::Reader, AssetLoader, LoadContext},
	prelude::*,
};
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pose {
	pub translation: [f32; 3],
	pub rotation: [f32; 4],
	pub scale: [f32; 3],
	pub cutoff: f32,
}
impl Pose {
	pub fn transform(&self) -> Transform {
		Transform {
			translation: Vec3::from_array(self.translation),
			rotation: Quat::from_array(self.rotation),
			scale: Vec3::from_array(self.scale),
		}
	}
	fn interpolate(&self, b: &Self, t: f32) -> Self {
		let a = self.transform();
		let b_t = b.transform();
		Self {
			translation: a.translation.lerp(b_t.translation, t).to_array(),
			rotation: a.rotation.slerp(b_t.rotation, t).to_array(),
			scale: a.scale.lerp(b_t.scale, t).to_array(),
			cutoff: self.cutoff + (b.cutoff - self.cutoff) * t,
		}
	}
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
	pub mesh: String,
	pub seed: u32,
	pub birth: f32,
	pub lifetime: f32,
	pub frames: Vec<Option<Pose>>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceStyle {
	pub light: [f32; 3],
	pub thresholds: [f32; 2],
	pub colors: [[f32; 3]; 3],
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshClipManifest {
	pub surface: SurfaceStyle,
	pub schema_version: u32,
	pub fps: f32,
	pub duration: f32,
	pub tracks: Vec<Track>,
}
impl MeshClipManifest {
	pub fn from_json(bytes: &[u8]) -> Result<Self, PackagedAssetError> {
		let m: Self = serde_json::from_slice(bytes)?;
		m.validate()?;
		Ok(m)
	}
	pub fn validate(&self) -> Result<(), PackagedAssetError> {
		let invalid =
			|| PackagedAssetError::invalid("invalid mesh clip timing, paths or bounded pose data");
		if self.schema_version != 1
			|| !self.fps.is_finite()
			|| self.fps <= 0.
			|| !self.duration.is_finite()
			|| self.duration <= 0.
			|| self.tracks.is_empty()
			|| self.tracks.len() > 64
		{
			return Err(invalid());
		}
		if !self
			.surface
			.light
			.iter()
			.chain(self.surface.thresholds.iter())
			.chain(self.surface.colors.iter().flatten())
			.all(|x| x.is_finite())
			|| Vec3::from_array(self.surface.light).length_squared() < 1e-6
			|| self.surface.thresholds[0] >= self.surface.thresholds[1]
		{
			return Err(invalid());
		}
		let intervals = self.duration * self.fps;
		if !intervals.is_finite()
			|| !(1.0..=1023.0).contains(&intervals)
			|| (intervals - intervals.round()).abs() > 0.001
		{
			return Err(invalid());
		}
		let count = intervals.round() as usize + 1;
		if !(2..=1024).contains(&count) {
			return Err(invalid());
		}
		for t in &self.tracks {
			if t.frames.len() != count
				|| t.mesh.starts_with('/')
				|| t.mesh.contains("..")
				|| !t.mesh.contains(".glb#Mesh")
				|| !t.birth.is_finite()
				|| t.birth < 0.
				|| !t.lifetime.is_finite()
				|| t.lifetime <= 0.
				|| t.birth + t.lifetime > self.duration + 1e-4
			{
				return Err(invalid());
			}
			for p in t.frames.iter().flatten() {
				let transform = p.transform();
				if !transform.translation.is_finite()
					|| !transform.scale.is_finite()
					|| transform.scale.min_element() <= 0.
					|| !transform.rotation.is_finite()
					|| (transform.rotation.length() - 1.).abs() > 0.001
					|| !p.cutoff.is_finite()
				{
					return Err(invalid());
				}
			}
		}
		Ok(())
	}
	pub fn sample(&self, track: usize, age: f32) -> Option<Pose> {
		if !age.is_finite() {
			return None;
		}
		let t = self.tracks.get(track)?;
		if age < t.birth || age >= t.birth + t.lifetime {
			return None;
		}
		let f = (age * self.fps).clamp(0., (t.frames.len() - 1) as f32);
		let i = f.floor() as usize;
		match (t.frames[i].as_ref(), t.frames.get(i + 1).and_then(|p| p.as_ref())) {
			(Some(a), Some(b)) => Some(a.interpolate(b, f.fract())),
			(Some(a), None) => Some(a.clone()),
			_ => None,
		}
	}
}
#[derive(Asset, TypePath, Clone, Debug)]
pub struct MeshClip {
	pub manifest: MeshClipManifest,
	#[dependency]
	pub meshes: Vec<Handle<Mesh>>,
}
#[derive(Default, TypePath)]
pub struct MeshClipLoader;
impl AssetLoader for MeshClipLoader {
	type Asset = MeshClip;
	type Settings = ();
	type Error = PackagedAssetError;
	async fn load(
		&self,
		reader: &mut dyn Reader,
		_: &(),
		ctx: &mut LoadContext<'_>,
	) -> Result<MeshClip, Self::Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let manifest = MeshClipManifest::from_json(&bytes)?;
		let meshes = manifest.tracks.iter().map(|t| ctx.load(t.mesh.clone())).collect();
		Ok(MeshClip { manifest, meshes })
	}
	fn extensions(&self) -> &[&str] {
		&["mesh_clip.json"]
	}
}
