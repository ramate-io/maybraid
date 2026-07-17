//! Optional debug visualization of Terrain / jersey cell bounds and a cell HUD.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use durham_terrain_models::{
	cascade_chunk_for_cell, JerseyFamilySummary, JerseyModulations, JerseyStampCellLayout, Terrain,
	TerrainCellLayout, TerrainEntryStore,
};
use lod::gen::Id;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::WorldBaseTerrain;

/// Half-height of surface-fitted jersey boxes (world units).
const JERSEY_BOX_HALF_HEIGHT: f32 = 30.0;

/// Playground debug overlays (wire bounds + cell HUD). Off by default.
#[derive(Resource, Debug, Clone, Copy)]
pub struct PlaygroundDebugOverlay {
	pub show_bounds: bool,
	pub show_cell_hud: bool,
	pub log_cell_changes: bool,
}

impl Default for PlaygroundDebugOverlay {
	fn default() -> Self {
		Self {
			show_bounds: false,
			show_cell_hud: false,
			log_cell_changes: false,
		}
	}
}

#[derive(Component)]
pub(crate) struct CellLocationHudText;

#[derive(Component)]
pub(crate) struct CellLocationHudRoot;

#[derive(Resource, Default)]
pub(crate) struct LastLoggedCellLocation {
	terrain: Option<(i32, i32)>,
	jersey: Option<(i32, i32)>,
	jersey_loaded: Option<bool>,
	terrain_present: Option<bool>,
}

/// Spawn a small top-of-screen panel for camera cell / stamp inspection (hidden by default).
pub fn setup_cell_location_hud(mut commands: Commands) {
	commands.insert_resource(LastLoggedCellLocation::default());
	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(8.0),
				left: Val::Px(8.0),
				right: Val::Px(8.0),
				padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(2.0),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.08, 0.12, 0.82)),
			Visibility::Hidden,
			CellLocationHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("cell HUD"),
				TextFont {
					font_size: bevy::text::FontSize::Px(12.0),
					..default()
				},
				TextColor(Color::srgb(0.92, 0.96, 1.0)),
				CellLocationHudText,
			));
		});
}

/// Draw wire AABBs when [`PlaygroundDebugOverlay::show_bounds`] is on.
pub fn draw_chunk_boundary_boxes(
	mut gizmos: Gizmos,
	overlay: Res<PlaygroundDebugOverlay>,
	terrains: Query<&Terrain>,
	layout: Res<TerrainCellLayout>,
	jersey_layout: Res<JerseyStampCellLayout>,
	base: Res<WorldBaseTerrain>,
) {
	if !overlay.show_bounds {
		return;
	}

	let terrain_color = Color::srgb(1.0, 0.2, 0.25);
	for terrain in &terrains {
		let chunk = cascade_chunk_for_cell(terrain.cell, terrain.res_2);
		let extent = chunk.extent_vec();
		let aabb = Aabb3d::from_min_max(chunk.origin, chunk.origin + extent);
		gizmos.aabb_3d(aabb, Transform::IDENTITY, terrain_color);
	}

	let jersey_color = Color::srgb(0.2, 0.85, 1.0);
	let region = layout.request_region();
	let grid_region = jersey_layout.region_in_grid_space(region);
	let size = jersey_layout.cell_size.max(1e-3);
	let min_x = (grid_region.min.x / size).floor() as i32;
	let max_x = (grid_region.max.x / size).ceil() as i32 - 1;
	let min_z = (grid_region.min.z / size).floor() as i32;
	let max_z = (grid_region.max.z / size).ceil() as i32 - 1;
	for ix in min_x..=max_x {
		for iz in min_z..=max_z {
			let cell = jersey_layout.cell_bounds(ix, iz);
			let aabb = surface_footprint_box(&cell, &base.0);
			gizmos.aabb_3d(aabb, Transform::IDENTITY, jersey_color);
		}
	}
}

/// Update the top HUD with terrain / jersey cell under the camera.
pub fn update_cell_location_hud(
	overlay: Res<PlaygroundDebugOverlay>,
	cameras: Query<&GlobalTransform, With<Camera3d>>,
	layout: Res<TerrainCellLayout>,
	jersey_layout: Res<JerseyStampCellLayout>,
	store: Res<TerrainEntryStore>,
	terrains: Query<&Terrain>,
	mut hud_root: Query<&mut Visibility, With<CellLocationHudRoot>>,
	mut hud: Query<&mut Text, With<CellLocationHudText>>,
	mut last: ResMut<LastLoggedCellLocation>,
) {
	if let Ok(mut visibility) = hud_root.single_mut() {
		*visibility = if overlay.show_cell_hud {
			Visibility::Visible
		} else {
			Visibility::Hidden
		};
	}

	if !overlay.show_cell_hud && !overlay.log_cell_changes {
		return;
	}

	let Ok(camera) = cameras.single() else {
		return;
	};

	let p = camera.translation();
	let (tix, tiz) = terrain_cell_coords(&layout, p);
	let (jix, jiz) = jersey_cell_coords(&jersey_layout, p);

	let t_size = layout.cell_size.max(1e-3);
	let t_cell = terrain_cell_aabb(tix, tiz, t_size, layout.vertical_half_extent);
	let j_cell = jersey_layout.cell_bounds(jix, jiz);
	let j_id = Id::from_cell(j_cell);

	let terrain = terrains.iter().find(|t| cells_match_xz(&t.cell, &t_cell));
	let jersey = store.jersey_modulation(j_id);

	let report = CellLocationReport {
		cam: p,
		terrain_ix: tix,
		terrain_iz: tiz,
		terrain_layout_size: t_size,
		terrain_cell: t_cell,
		terrain: terrain.map(TerrainReport::from_terrain),
		jersey_ix: jix,
		jersey_iz: jiz,
		jersey_layout_size: jersey_layout.cell_size,
		jersey_origin_offset: jersey_layout.origin_offset,
		jersey_cell: j_cell,
		jersey: jersey.map(JerseyReport::from_mods),
	};

	let rendered = report.to_string();
	if overlay.show_cell_hud {
		if let Ok(mut text) = hud.single_mut() {
			*text = Text::new(rendered.clone());
		}
	}

	if !overlay.log_cell_changes {
		return;
	}

	let terrain_present = report.terrain.is_some();
	let jersey_loaded = report.jersey.is_some();
	let changed = last.terrain != Some((tix, tiz))
		|| last.jersey != Some((jix, jiz))
		|| last.terrain_present != Some(terrain_present)
		|| last.jersey_loaded != Some(jersey_loaded);
	if changed {
		last.terrain = Some((tix, tiz));
		last.jersey = Some((jix, jiz));
		last.terrain_present = Some(terrain_present);
		last.jersey_loaded = Some(jersey_loaded);
		info!("\n{rendered}");
	}
}

#[derive(Debug, Clone)]
struct CellLocationReport {
	cam: Vec3,
	terrain_ix: i32,
	terrain_iz: i32,
	terrain_layout_size: f32,
	terrain_cell: Aabb3d,
	terrain: Option<TerrainReport>,
	jersey_ix: i32,
	jersey_iz: i32,
	jersey_layout_size: f32,
	jersey_origin_offset: Vec2,
	jersey_cell: Aabb3d,
	jersey: Option<JerseyReport>,
}

#[derive(Debug, Clone)]
struct TerrainReport {
	cell: Aabb3d,
	chunk: Aabb3d,
	res_2: u8,
	stamps: Vec<JerseyReport>,
}

#[derive(Debug, Clone)]
struct JerseyReport {
	cell: Aabb3d,
	ops: usize,
	families: Vec<JerseyFamilySummary>,
}

impl TerrainReport {
	fn from_terrain(t: &Terrain) -> Self {
		let chunk = cascade_chunk_for_cell(t.cell, t.res_2);
		let extent = chunk.extent_vec();
		Self {
			cell: t.cell,
			chunk: Aabb3d::from_min_max(chunk.origin, chunk.origin + extent),
			res_2: t.res_2,
			stamps: t.jersey.iter().map(JerseyReport::from_mods).collect(),
		}
	}
}

impl JerseyReport {
	fn from_mods(m: &JerseyModulations) -> Self {
		Self {
			cell: m.cell,
			ops: m.modulations.len(),
			families: m.families.clone(),
		}
	}
}

impl Display for CellLocationReport {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		writeln!(f, "── camera cell report ──────────────────────────────")?;
		writeln!(
			f,
			"cam          {:?}",
			(self.cam.x, self.cam.y, self.cam.z)
		)?;
		writeln!(f)?;
		writeln!(
			f,
			"terrain      T({}, {})  layout_size={:.3}",
			self.terrain_ix, self.terrain_iz, self.terrain_layout_size
		)?;
		writeln!(f, "  cell AABB  {}", fmt_aabb(&self.terrain_cell))?;
		match &self.terrain {
			Some(t) => {
				writeln!(f, "  status     GENERATED  res_2={}", t.res_2)?;
				writeln!(f, "  cell AABB  {}", fmt_aabb(&t.cell))?;
				writeln!(f, "  chunk AABB {}", fmt_aabb(&t.chunk))?;
				writeln!(f, "  stamps     n={}", t.stamps.len())?;
				if t.stamps.is_empty() {
					writeln!(f, "    (none composed onto this Terrain)")?;
				}
				for (i, stamp) in t.stamps.iter().enumerate() {
					writeln!(
						f,
						"    [{i}] ops={}  families={:?}",
						stamp.ops, stamp.families
					)?;
					writeln!(f, "        AABB {}", fmt_aabb(&stamp.cell))?;
				}
			}
			None => writeln!(f, "  status     NOT GENERATED")?,
		}
		writeln!(f)?;
		writeln!(
			f,
			"jersey       J({}, {})  layout_size={:.3}  origin_offset={:?}",
			self.jersey_ix,
			self.jersey_iz,
			self.jersey_layout_size,
			(
				self.jersey_origin_offset.x,
				self.jersey_origin_offset.y
			)
		)?;
		writeln!(f, "  cell AABB  {}", fmt_aabb(&self.jersey_cell))?;
		match &self.jersey {
			Some(j) => {
				writeln!(f, "  status     LOADED  ops={}", j.ops)?;
				writeln!(f, "  families   {:?}", j.families)?;
				writeln!(f, "  store AABB {}", fmt_aabb(&j.cell))?;
			}
			None => writeln!(f, "  status     NOT LOADED")?,
		}
		write!(f, "───────────────────────────────────────────────────")
	}
}

fn fmt_aabb(a: &Aabb3d) -> String {
	format!(
		"min=({:.3}, {:.3}, {:.3})  max=({:.3}, {:.3}, {:.3})",
		a.min.x, a.min.y, a.min.z, a.max.x, a.max.y, a.max.z
	)
}

fn terrain_cell_aabb(ix: i32, iz: i32, cell_size: f32, vertical_half_extent: f32) -> Aabb3d {
	let size = cell_size.max(1e-3);
	let vy = vertical_half_extent.max(size);
	Aabb3d::from_min_max(
		Vec3::new(ix as f32 * size, -vy, iz as f32 * size),
		Vec3::new((ix + 1) as f32 * size, vy, (iz + 1) as f32 * size),
	)
}

fn terrain_cell_coords(layout: &TerrainCellLayout, p: Vec3) -> (i32, i32) {
	let s = layout.cell_size.max(1e-3);
	((p.x / s).floor() as i32, (p.z / s).floor() as i32)
}

fn jersey_cell_coords(layout: &JerseyStampCellLayout, p: Vec3) -> (i32, i32) {
	let s = layout.cell_size.max(1e-3);
	let gx = p.x - layout.origin_offset.x;
	let gz = p.z - layout.origin_offset.y;
	((gx / s).floor() as i32, (gz / s).floor() as i32)
}

fn cells_match_xz(a: &Aabb3d, b: &Aabb3d) -> bool {
	(a.min.x - b.min.x).abs() < 1e-3
		&& (a.min.z - b.min.z).abs() < 1e-3
		&& (a.max.x - b.max.x).abs() < 1e-3
		&& (a.max.z - b.max.z).abs() < 1e-3
}

fn surface_footprint_box(
	cell: &Aabb3d,
	base: &durham_terrain_models::BaseTerrainNoise,
) -> Aabb3d {
	let min = Vec3::from(cell.min);
	let max = Vec3::from(cell.max);
	let cx = (min.x + max.x) * 0.5;
	let cz = (min.z + max.z) * 0.5;
	let y = base.height_at(cx, cz);
	Aabb3d::from_min_max(
		Vec3::new(min.x, y - JERSEY_BOX_HALF_HEIGHT, min.z),
		Vec3::new(max.x, y + JERSEY_BOX_HALF_HEIGHT, max.z),
	)
}
