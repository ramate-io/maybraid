//! Shared 2D tile [`Material2d`]: subdivided quad, world-space shade, thematic marks.

use bevy::{
	asset::{embedded_asset, RenderAssetUsages},
	mesh::Indices,
	prelude::*,
	reflect::TypePath,
	render::render_resource::{AsBindGroup, PrimitiveTopology, ShaderType},
	shader::ShaderRef,
	sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

use crate::map::SkillKind;
use crate::tiles::TileKind;

pub const TILE_KIND_LAND: f32 = 0.0;
pub const TILE_KIND_WATER: f32 = 1.0;
pub const TILE_KIND_FIRE: f32 = 2.0;
pub const TILE_KIND_WAVE: f32 = 3.0;
pub const TILE_KIND_CURSOR: f32 = 4.0;
const TILE_DIVISIONS: u32 = 8;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TileParams {
	pub tint: Vec4,
	/// `x` kind, `y` intensity, `z` seed.
	pub style: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkillMapTileMaterial {
	#[uniform(0)]
	pub params: TileParams,
}

impl SkillMapTileMaterial {
	pub fn for_kind(kind: TileKind, seed: u32) -> Self {
		let (tint, code) = match kind {
			TileKind::Land => (Vec4::new(1.0, 0.96, 0.9, 1.0), TILE_KIND_LAND),
			TileKind::Water => (Vec4::new(1.0, 1.0, 1.05, 1.0), TILE_KIND_WATER),
			TileKind::Power(SkillKind::Fireball) => {
				(Vec4::new(1.05, 0.95, 0.88, 1.0), TILE_KIND_FIRE)
			}
			TileKind::Power(SkillKind::Dumbwave) => {
				(Vec4::new(0.95, 1.02, 1.08, 1.0), TILE_KIND_WAVE)
			}
		};
		Self { params: TileParams { tint, style: Vec4::new(code, 1.0, seed as f32, 0.0) } }
	}

	pub fn cursor() -> Self {
		Self {
			params: TileParams {
				tint: Vec4::ONE,
				style: Vec4::new(TILE_KIND_CURSOR, 1.0, 0.0, 0.0),
			},
		}
	}
}

impl Material2d for SkillMapTileMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "tile_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "tile_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode2d {
		AlphaMode2d::Opaque
	}
}

/// Shared subdivided quad plus one material per tile class.
#[derive(Resource, Clone)]
pub struct SkillMapTileAssets {
	pub mesh: Handle<Mesh>,
	pub cursor_mesh: Handle<Mesh>,
	pub land: Handle<SkillMapTileMaterial>,
	pub water: Handle<SkillMapTileMaterial>,
	pub fireball: Handle<SkillMapTileMaterial>,
	pub dumbwave: Handle<SkillMapTileMaterial>,
	pub cursor: Handle<SkillMapTileMaterial>,
}

impl SkillMapTileAssets {
	pub fn material(&self, kind: TileKind) -> Handle<SkillMapTileMaterial> {
		match kind {
			TileKind::Land => self.land.clone(),
			TileKind::Water => self.water.clone(),
			TileKind::Power(SkillKind::Fireball) => self.fireball.clone(),
			TileKind::Power(SkillKind::Dumbwave) => self.dumbwave.clone(),
		}
	}
}

pub struct SkillMapTileMaterialPlugin;

impl Plugin for SkillMapTileMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "tile_material.wgsl");
		app.add_plugins(Material2dPlugin::<SkillMapTileMaterial>::default())
			.add_systems(Startup, setup_tile_assets);
	}
}

fn setup_tile_assets(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<SkillMapTileMaterial>>,
) {
	let mesh = meshes.add(subdivided_quad(16.0, 16.0, TILE_DIVISIONS));
	let cursor_mesh = meshes.add(Circle::new(4.6).mesh().resolution(28).build());
	commands.insert_resource(SkillMapTileAssets {
		mesh,
		cursor_mesh,
		land: materials.add(SkillMapTileMaterial::for_kind(TileKind::Land, 1)),
		water: materials.add(SkillMapTileMaterial::for_kind(TileKind::Water, 2)),
		fireball: materials
			.add(SkillMapTileMaterial::for_kind(TileKind::Power(SkillKind::Fireball), 3)),
		dumbwave: materials
			.add(SkillMapTileMaterial::for_kind(TileKind::Power(SkillKind::Dumbwave), 4)),
		cursor: materials.add(SkillMapTileMaterial::cursor()),
	});
}

/// Grid of quads in XY, UV 0..1, centered on the origin.
pub fn subdivided_quad(width: f32, height: f32, divisions: u32) -> Mesh {
	let divisions = divisions.max(1);
	let cols = divisions + 1;
	let hw = width * 0.5;
	let hh = height * 0.5;
	let mut positions = Vec::with_capacity((cols * cols) as usize);
	let mut normals = Vec::with_capacity((cols * cols) as usize);
	let mut uvs = Vec::with_capacity((cols * cols) as usize);
	for y in 0..cols {
		let v = y as f32 / divisions as f32;
		for x in 0..cols {
			let u = x as f32 / divisions as f32;
			positions.push([u * width - hw, v * height - hh, 0.0]);
			normals.push([0.0, 0.0, 1.0]);
			uvs.push([u, 1.0 - v]);
		}
	}
	let mut indices = Vec::with_capacity((divisions * divisions * 6) as usize);
	for y in 0..divisions {
		for x in 0..divisions {
			let i = y * cols + x;
			indices.extend([i, i + 1, i + cols, i + 1, i + cols + 1, i + cols]);
		}
	}
	Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
		.with_inserted_indices(Indices::U32(indices))
		.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
		.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
		.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn subdivided_quad_has_more_than_a_sprite() {
		let mesh = subdivided_quad(16.0, 16.0, 8);
		assert_eq!(mesh.count_vertices(), 81);
	}

	#[test]
	fn power_kinds_use_thematic_codes() {
		let fire = SkillMapTileMaterial::for_kind(TileKind::Power(SkillKind::Fireball), 0);
		let wave = SkillMapTileMaterial::for_kind(TileKind::Power(SkillKind::Dumbwave), 0);
		assert_eq!(fire.params.style.x, TILE_KIND_FIRE);
		assert_eq!(wave.params.style.x, TILE_KIND_WAVE);
		assert_eq!(SkillMapTileMaterial::cursor().params.style.x, TILE_KIND_CURSOR);
	}
}
