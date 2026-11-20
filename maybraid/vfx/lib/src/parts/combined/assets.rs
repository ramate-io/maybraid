//! Typed runtime dependencies; no source delivery or recorded trajectories at runtime.
use crate::parts::{
	combined::simulation::Config,
	packaged::{Bounds3, PackagedAssetError, StaticVolume},
};
use bevy::{
	asset::{io::Reader, AssetLoader, LoadContext},
	prelude::*,
};
use serde::Deserialize;
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Anchor {
	pub point: [f32; 3],
	pub normal: [f32; 3],
}
#[derive(Clone, Debug, Deserialize)]
pub struct Style {
	pub color_linear_rgb: [f32; 3],
	pub extinction_per_m: f32,
	pub concentration_variation: f32,
	pub ramp_end_density: f32,
	pub overlap_cap: f32,
	pub offline_light_contrast: f32,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
	pub schema_version: u32,
	pub bodies: [String; 3],
	pub clouds: [String; 4],
	pub meshes: [String; 3],
	pub rules: String,
	pub body_metadata: String,
	pub cloud_library: String,
	pub anchors: [[Anchor; 3]; 3],
	pub rest_support: [Bounds3; 3],
}
#[derive(Asset, TypePath, Clone, Debug)]
pub struct CombinedAsset {
	pub manifest: Manifest,
	pub config: Config,
	pub style: Style,
	#[dependency]
	pub bodies: [Handle<StaticVolume>; 3],
	#[dependency]
	pub clouds: [Handle<StaticVolume>; 4],
	#[dependency]
	pub meshes: [Handle<Mesh>; 3],
}
#[derive(Default, TypePath)]
pub struct CombinedLoader;
impl AssetLoader for CombinedLoader {
	type Asset = CombinedAsset;
	type Settings = ();
	type Error = PackagedAssetError;
	async fn load(
		&self,
		reader: &mut dyn Reader,
		_: &(),
		ctx: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let m: Manifest = serde_json::from_slice(&bytes)?;
		if m.schema_version != 1 {
			return Err(PackagedAssetError::invalid("unsupported Combined manifest"));
		}
		for a in m.anchors.iter().flatten() {
			if !Vec3::from_array(a.point).is_finite()
				|| !(0.9..1.1).contains(&Vec3::from_array(a.normal).length())
			{
				return Err(PackagedAssetError::invalid("invalid rest anchor/normal"));
			}
		}
		for b in &m.rest_support {
			if !Vec3::from_array(b.min).is_finite()
				|| !Vec3::from_array(b.max).is_finite()
				|| !Vec3::from_array(b.max).cmpgt(Vec3::from_array(b.min)).all()
			{
				return Err(PackagedAssetError::invalid("invalid rest support"));
			}
		}
		let source = ctx.path().clone();
		let path = |p: &str| {
			source
				.resolve_embed_str(p)
				.map_err(|e| PackagedAssetError::invalid(e.to_string()))
		};
		let rules: serde_json::Value =
			serde_json::from_slice(&ctx.read_asset_bytes(path(&m.rules)?).await?)?;
		if rules["schema"] != "maybraid.dynamic-dust.v1" {
			return Err(PackagedAssetError::invalid("unsupported dynamic rules"));
		}
		let config: Config = serde_json::from_value(rules["config"].clone())?;
		config.validate()?;
		let meta: serde_json::Value =
			serde_json::from_slice(&ctx.read_asset_bytes(path(&m.body_metadata)?).await?)?;
		let style: Style = serde_json::from_value(meta["combined_shading"].clone())?;
		let cloud_meta: serde_json::Value =
			serde_json::from_slice(&ctx.read_asset_bytes(path(&m.cloud_library)?).await?)?;
		if cloud_meta["schema"] != "maybraid.small-cloud-library.v1"
			|| cloud_meta["assets"].as_array().is_none_or(|a| a.len() != 4)
		{
			return Err(PackagedAssetError::invalid("expected four cloud elements"));
		}
		if [
			style.extinction_per_m,
			style.ramp_end_density,
			style.overlap_cap,
			style.offline_light_contrast,
		]
		.iter()
		.any(|v| !v.is_finite() || *v <= 0.)
			|| !Vec3::from_array(style.color_linear_rgb).is_finite()
			|| !(0.0..=1.0).contains(&style.concentration_variation)
		{
			return Err(PackagedAssetError::invalid("invalid Combined appearance"));
		}
		let bodies = [
			ctx.load(path(&m.bodies[0])?),
			ctx.load(path(&m.bodies[1])?),
			ctx.load(path(&m.bodies[2])?),
		];
		let clouds = [
			ctx.load(path(&m.clouds[0])?),
			ctx.load(path(&m.clouds[1])?),
			ctx.load(path(&m.clouds[2])?),
			ctx.load(path(&m.clouds[3])?),
		];
		let meshes = [
			ctx.load(path(&m.meshes[0])?),
			ctx.load(path(&m.meshes[1])?),
			ctx.load(path(&m.meshes[2])?),
		];
		Ok(CombinedAsset { manifest: m, config, style, bodies, clouds, meshes })
	}
	fn extensions(&self) -> &[&str] {
		&["combined.json"]
	}
}
