//! Preview subject sync. Viewer tracking lives in [`lod::LodFinePassPlugin`].

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use lod::gen::LodScene;
use lod::LodViewerState;
use procedural_common::{AllowedAngles, NoiseParams, StepLenRange};
use richmond_building_components::panels::{PanelGeometry, PanelNode, TessellatedTriangle};
use richmond_building_components::partitions::rough_stonework::{
	RoughStonework180, RoughStonework90, RoughStoneworkLinear, RoughStoneworkSlice90,
};
use richmond_building_components::partitions::{Partition, PartitionNode};
use richmond_building_components::roofs::{Pitch, RoofGeometry, RoofNode};
use richmond_building_components::Placement;
use richmond_building_components::ComponentsOnly;
use richmond_buildings::bedroom::Bedroom;
use richmond_buildings::panel_complex::{PanelComplex, PanelComplexJointPolicy, PanelPoint};
use richmond_buildings::quad_panel::QuadPanel;
use richmond_buildings::quad_panel_complex::QuadPanelComplex;
use richmond_buildings::ruled_pitch::RuledPitch;
use richmond_buildings::stacked_rings::StackedRings;
use richmond_buildings::tessellated_triangle_panel::TessellatedTrianglePanel;
use richmond_buildings::walling::{
	LinearWall, LinearWallParams, MustAssignPortal, NoisyPolylineWall, NoisyPolylineWallParams,
	PolylineWall, PolylineWallParams, Portal, Walling,
};
use richmond_buildings::wizards_tower::WizardsTower;
use richmond_buildings::{
	BedroomFillParams, CellConstraints, CirculationEntry, CirculationRequestStatus,
};

#[derive(Component)]
pub struct PreviewRoot;

#[derive(Clone, Debug, PartialEq)]
pub enum PreviewSubject {
	None,
	Linear,
	Arc90,
	Arc180,
	Slice90,
	Pitch {
		rise: f32,
		run: f32,
		length: Option<f32>,
		tile_width: f32,
		left: Option<f32>,
		right: Option<f32>,
	},
	TessellatedTriangle {
		a: Vec2,
		b: Vec2,
		c: Vec2,
	},
	TessellatedTriangle3d {
		a: Vec3,
		b: Vec3,
		c: Vec3,
	},
	QuadPanel {
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		t_a0: f32,
		t_a1: f32,
		t_b0: f32,
		t_b1: f32,
		min_dihedral: f32,
		no_joint: bool,
	},
	PanelComplex {
		mesh: String,
		min_dihedral: f32,
		no_joint: bool,
	},
	QuadPanelComplex {
		mesh: String,
		min_dihedral: f32,
		no_joint: bool,
	},
	RuledPitch {
		min_dihedral: f32,
		no_joint: bool,
	},
	Polyline,
	LinearWall,
	PolylineWall,
	NoisyPolylineWall {
		distance: f32,
		step_len: StepLenRange,
		allowed_angles: AllowedAngles,
		path_noise: NoiseParams,
	},
	WizardsTower {
		noise: f32,
	},
	StackedRings {
		floor_count: u32,
		floor_height: f32,
		radius: f32,
	},
	Bedroom {
		/// Cell size along X / Y / Z (AABB from origin to `extent`).
		extent: Vec3,
		/// Unit noise for layout fitting.
		noise: f32,
		spaciousness: f32,
		occupancy: f32,
		/// When true, add a required −Z door circulation region.
		door: bool,
	},
}

impl Default for PreviewSubject {
	fn default() -> Self {
		Self::None
	}
}

#[derive(Resource, Clone, Debug)]
pub struct PreviewConfig {
	pub subject: PreviewSubject,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { subject: PreviewSubject::None, transform: Transform::IDENTITY }
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		match self.subject {
			PreviewSubject::None => "preview: (none — `/show …`)".into(),
			PreviewSubject::Linear => "preview: rough-stonework linear".into(),
			PreviewSubject::Arc90 => "preview: rough-stonework arc-90".into(),
			PreviewSubject::Arc180 => "preview: rough-stonework arc-180".into(),
			PreviewSubject::Slice90 => "preview: rough-stonework slice-90".into(),
			PreviewSubject::Pitch {
				rise,
				run,
				length,
				tile_width,
				left,
				right,
			} => {
				format!(
					"preview: pitch (rise={rise:.2} run={run:.2} len={length:?} tile={tile_width:.2} left={left:?} right={right:?})"
				)
			}
			PreviewSubject::TessellatedTriangle { a, b, c } => {
				format!("preview: tessellated-triangle (a={a:?} b={b:?} c={c:?})")
			}
			PreviewSubject::TessellatedTriangle3d { a, b, c } => {
				format!("preview: tessellated-triangle-3d (a={a:?} b={b:?} c={c:?})")
			}
			PreviewSubject::QuadPanel {
				a0,
				a1,
				b0,
				b1,
				t_a0,
				t_a1,
				t_b0,
				t_b1,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: quad-panel (a0={a0:?} a1={a1:?} b0={b0:?} b1={b1:?} t=[{t_a0:.2},{t_a1:.2},{t_b0:.2},{t_b1:.2}] min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::PanelComplex {
				ref mesh,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: panel-complex (mesh={mesh:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::QuadPanelComplex {
				ref mesh,
				min_dihedral,
				no_joint,
			} => format!(
				"preview: quad-panel-complex (mesh={mesh:?} min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::RuledPitch {
				min_dihedral,
				no_joint,
			} => format!(
				"preview: ruled-pitch (min_dihedral={min_dihedral:.3} no_joint={no_joint})"
			),
			PreviewSubject::Polyline => "preview: partition polyline (L)".into(),
			PreviewSubject::LinearWall => "preview: walling linear-wall (door)".into(),
			PreviewSubject::PolylineWall => "preview: walling polyline-wall (door)".into(),
			PreviewSubject::NoisyPolylineWall {
				distance,
				step_len,
				allowed_angles,
				path_noise,
			} => format!(
				"preview: noisy-polyline-wall (d={distance:.1} step=[{:.2},{:.2}] ang=({:.2},{:.2},{:.2}) seed={})",
				step_len.min, step_len.max,
				allowed_angles.x, allowed_angles.y, allowed_angles.z, path_noise.seed
			),
			PreviewSubject::WizardsTower { noise } => {
				format!("preview: wizards-tower (noise={noise:.2})")
			}
			PreviewSubject::StackedRings {
				floor_count,
				floor_height,
				radius,
			} => format!(
				"preview: stacked-rings (n={floor_count} h={floor_height:.2} r={radius:.2})"
			),
			PreviewSubject::Bedroom {
				extent,
				noise,
				spaciousness,
				occupancy,
				door,
			} => {
				format!(
					"preview: bedroom (extent={:.2},{:.2},{:.2} noise={noise:.2} space={spaciousness:.2} occ={occupancy:.2} door={door})",
					extent.x, extent.y, extent.z
				)
			}
		}
	}

	fn subject_bounds(&self) -> Aabb3d {
		match &self.subject {
			PreviewSubject::StackedRings { radius, floor_count, floor_height } => {
				let r = (*radius).max(1e-4);
				let h = (*floor_count as f32) * (*floor_height).max(1e-4);
				Aabb3d::from_min_max(Vec3::new(-r, 0.0, -r), Vec3::new(r, h, r))
			}
			PreviewSubject::WizardsTower { .. } => {
				Aabb3d::from_min_max(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 3.0, 4.0))
			}
			PreviewSubject::Bedroom { extent, .. } => Aabb3d::from_min_max(Vec3::ZERO, *extent),
			PreviewSubject::Pitch { rise, run, length, left, right, .. } => {
				let left_w = left.map(|b| b.abs()).unwrap_or(0.0);
				let right_w = right.map(|b| b.abs()).unwrap_or(0.0);
				let len = length.unwrap_or(0.0);
				let x_max = (left_w + len + right_w).max(1e-4);
				let run = (*run).max(1e-4);
				let rise = (*rise).max(0.0);
				Aabb3d::from_min_max(Vec3::new(0.0, -0.2, -run), Vec3::new(x_max, rise + 0.2, 0.0))
			}
			PreviewSubject::TessellatedTriangle { a, b, c, .. } => {
				let min_x = a.x.min(b.x).min(c.x) - 0.2;
				let max_x = a.x.max(b.x).max(c.x) + 0.2;
				let min_z = a.y.min(b.y).min(c.y) - 0.2;
				let max_z = a.y.max(b.y).max(c.y) + 0.2;
				Aabb3d::from_min_max(Vec3::new(min_x, -0.2, min_z), Vec3::new(max_x, 0.2, max_z))
			}
			PreviewSubject::TessellatedTriangle3d { a, b, c } => {
				let min = a.min(*b).min(*c) - Vec3::splat(0.2);
				let max = a.max(*b).max(*c) + Vec3::splat(0.2);
				Aabb3d::from_min_max(min, max)
			}
			PreviewSubject::QuadPanel { a0, a1, b0, b1, .. } => {
				let min = a0.min(*a1).min(*b0).min(*b1) - Vec3::splat(0.2);
				let max = a0.max(*a1).max(*b0).max(*b1) + Vec3::splat(0.2);
				Aabb3d::from_min_max(min, max)
			}
			PreviewSubject::Polyline | PreviewSubject::PolylineWall => {
				Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 3.0, 4.0))
			}
			PreviewSubject::LinearWall => {
				Aabb3d::from_min_max(Vec3::new(-4.0, 0.0, -0.5), Vec3::new(4.0, 3.0, 0.5))
			}
			PreviewSubject::NoisyPolylineWall { distance, .. } => {
				let r = (*distance).max(4.0);
				Aabb3d::from_min_max(Vec3::new(-r, -r * 0.5, -r), Vec3::new(r, r * 0.5 + 3.0, r))
			}
			_ => Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
		}
	}
}

/// Authored preview payload kept across LOD flips (stable noise / geometry).
#[derive(Resource, Default)]
pub struct CachedPreview {
	key: Option<(PreviewSubject, Transform)>,
	wizards_tower: Option<WizardsTower>,
	stacked_rings: Option<StackedRings>,
	bedroom: Option<Bedroom>,
	walling: Option<Walling>,
}

impl CachedPreview {
	fn rebuild_if_needed(&mut self, config: &PreviewConfig) {
		let key = (config.subject.clone(), config.transform);
		if self.key.as_ref() == Some(&key) {
			return;
		}
		self.key = Some(key);
		self.wizards_tower = None;
		self.stacked_rings = None;
		self.bedroom = None;
		self.walling = None;
		match &config.subject {
			PreviewSubject::WizardsTower { noise } => {
				let footprint = CellConstraints::cell_owned(Aabb3d::from_min_max(
					Vec3::new(-4.0, 0.0, -4.0),
					Vec3::new(4.0, 3.0, 4.0),
				));
				self.wizards_tower = Some(WizardsTower::new(&footprint, *noise));
			}
			PreviewSubject::StackedRings { floor_count, floor_height, radius } => {
				self.stacked_rings = Some(StackedRings::new(*floor_count, *floor_height, *radius));
			}
			PreviewSubject::Bedroom { extent, noise, spaciousness, occupancy, door } => {
				let mut room =
					CellConstraints::cell_owned(Aabb3d::from_min_max(Vec3::ZERO, *extent));
				if *door {
					room.circulation.front = Some(CirculationEntry(vec![(
						Aabb2d { min: Vec2::new(0.35, 0.0), max: Vec2::new(0.65, 0.9) },
						vec![CirculationRequestStatus::Required],
					)]));
				}
				self.bedroom = Some(Bedroom::with_fill(
					room,
					*noise,
					BedroomFillParams { spaciousness: *spaciousness, occupancy: *occupancy },
				));
			}
			PreviewSubject::LinearWall => {
				self.walling = Some(Walling::Linear(LinearWall::new(LinearWallParams {
					start: Vec3::new(-4.0, 0.0, 0.0),
					end: Vec3::new(4.0, 0.0, 0.0),
					height: 3.0,
					must_assign: vec![MustAssignPortal::at(0.5, Portal::Door)],
					optional_portals: (0, 0),
					..LinearWallParams::default()
				})));
			}
			PreviewSubject::PolylineWall => {
				self.walling = Some(Walling::Polyline(PolylineWall::new(PolylineWallParams {
					points: vec![
						Vec3::new(0.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 4.0),
					],
					height: 3.0,
					must_assign: vec![MustAssignPortal::at(0.25, Portal::Door)],
					optional_portals: (0, 0),
					..PolylineWallParams::default()
				})));
			}
			PreviewSubject::NoisyPolylineWall {
				distance,
				step_len,
				allowed_angles,
				path_noise,
			} => {
				self.walling =
					Some(Walling::NoisyPolyline(NoisyPolylineWall::new(NoisyPolylineWallParams {
						distance: *distance,
						step_len: *step_len,
						allowed_angles: *allowed_angles,
						path_noise: *path_noise,
						optional_portals: (0, 0),
						..NoisyPolylineWallParams::default()
					})));
			}
			_ => {}
		}
	}
}

/// Spawn preview when the subject changes. LOD flips update host levels in-place
/// ([`lod::LodFinePassPlugin`] + domain fine-phase systems).
pub fn present_preview_lod(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	lod_state: Res<LodViewerState>,
	mut cache: ResMut<CachedPreview>,
	roots: Query<Entity, With<PreviewRoot>>,
	mut last_subject: Local<Option<(PreviewSubject, Transform)>>,
) {
	let subject_key = (config.subject.clone(), config.transform);
	let subject_changed = last_subject.as_ref() != Some(&subject_key);
	let has_root = roots.iter().next().is_some();

	if matches!(config.subject, PreviewSubject::None) {
		if subject_changed || has_root {
			for entity in &roots {
				commands.entity(entity).despawn();
			}
			*last_subject = Some(subject_key);
			cache.key = None;
			cache.wizards_tower = None;
			cache.stacked_rings = None;
			cache.bedroom = None;
			cache.walling = None;
		}
		return;
	}

	cache.rebuild_if_needed(&config);

	if !subject_changed && has_root {
		return;
	}

	for entity in &roots {
		commands.entity(entity).despawn();
	}
	*last_subject = Some(subject_key);

	let bounds = config.subject_bounds();
	let lod_ref = lod_state.lod_ref(&bounds);

	let transform = config.transform;
	match &config.subject {
		PreviewSubject::None => {}
		PreviewSubject::Linear => {
			spawn_preview(&mut commands, transform, RoughStoneworkLinear.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Arc90 => {
			spawn_preview(&mut commands, transform, RoughStonework90.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Arc180 => {
			spawn_preview(&mut commands, transform, RoughStonework180.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Slice90 => {
			spawn_preview(&mut commands, transform, RoughStoneworkSlice90.scene_with_lod(&lod_ref));
		}
		PreviewSubject::Pitch { rise, run, length, tile_width, left, right } => {
			let mut pitch = Pitch::new(*rise, *run, *tile_width);
			if let Some(len) = length {
				pitch = pitch.with_length(*len);
			}
			if let Some(base) = left {
				pitch = pitch.with_left(*base);
			}
			if let Some(base) = right {
				pitch = pitch.with_right(*base);
			}
			let roof = RoofNode::shepherds_thatch(RoofGeometry::pitch(pitch), Placement::IDENTITY);
			spawn_preview(&mut commands, transform, roof.scene_with_lod(&lod_ref));
		}
		PreviewSubject::TessellatedTriangle { a, b, c } => {
			let panel = PanelNode::rough_stone(
				PanelGeometry::tessellated_triangle(TessellatedTriangle::new(*a, *b, *c)),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, panel.scene_with_lod(&lod_ref));
		}
		PreviewSubject::TessellatedTriangle3d { a, b, c } => {
			let panel = TessellatedTrianglePanel::rough_stone(*a, *b, *c);
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(panel).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::QuadPanel {
			a0,
			a1,
			b0,
			b1,
			t_a0,
			t_a1,
			t_b0,
			t_b1,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			let complex = QuadPanel::rough_stone(
				PanelPoint::new(*a0, *t_a0),
				PanelPoint::new(*a1, *t_a1),
				PanelPoint::new(*b0, *t_b0),
				PanelPoint::new(*b1, *t_b1),
			)
			.with_joint_policy(policy)
			.into_complex();
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(complex).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::PanelComplex {
			mesh,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			match mesh.parse::<PanelComplex>() {
				Ok(complex) => {
					let complex = complex.with_joint_policy(policy);
					spawn_preview(
						&mut commands,
						transform,
						ComponentsOnly(complex).scene_with_lod(&lod_ref),
					);
				}
				Err(e) => {
					warn!("panel-complex parse failed: {e}");
				}
			}
		}
		PreviewSubject::QuadPanelComplex {
			mesh,
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			match mesh.parse::<QuadPanelComplex>() {
				Ok(quads) => {
					let complex = quads.with_joint_policy(policy).into_complex();
					spawn_preview(
						&mut commands,
						transform,
						ComponentsOnly(complex).scene_with_lod(&lod_ref),
					);
				}
				Err(e) => {
					warn!("quad-panel-complex parse failed: {e}");
				}
			}
		}
		PreviewSubject::RuledPitch {
			min_dihedral,
			no_joint,
		} => {
			let policy = if *no_joint {
				PanelComplexJointPolicy::never()
			} else {
				PanelComplexJointPolicy::min_dihedral_rad(*min_dihedral)
			};
			// Funky 5+5: eave snakes on the ground; ridge wanders higher with a lag,
			// so rafters twist and bays pick up visible crease dihedrals.
			let eave = [
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(0.6, 0.15, 1.4),
				Vec3::new(-0.3, 0.0, 2.8),
				Vec3::new(0.9, 0.25, 4.1),
				Vec3::new(0.2, 0.0, 5.6),
			];
			let ridge = [
				Vec3::new(1.8, 1.6, 0.4),
				Vec3::new(2.6, 2.1, 1.1),
				Vec3::new(1.4, 1.4, 2.5),
				Vec3::new(2.9, 2.4, 3.6),
				Vec3::new(2.1, 1.7, 5.2),
			];
			let complex = RuledPitch::shepherds_thatch()
				.with_lines(eave, ridge)
				.with_joint_policy(policy)
				.into_complex();
			spawn_preview(
				&mut commands,
				transform,
				ComponentsOnly(complex).scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Polyline => {
			let node = PartitionNode::rough_stone(
				Partition::Polyline(
					richmond_building_components::partitions::PolylinePartition::new([
						Vec3::new(0.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 0.0),
						Vec3::new(4.0, 0.0, 4.0),
					])
					.with_wall_scale(3.0, 1.0),
				),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, node.scene_with_lod(&lod_ref));
		}
		PreviewSubject::LinearWall
		| PreviewSubject::PolylineWall
		| PreviewSubject::NoisyPolylineWall { .. } => {
			if let Some(walling) = cache.walling.as_ref() {
				spawn_preview(
					&mut commands,
					transform,
					ComponentsOnly(walling).scene_with_lod(&lod_ref),
				);
			}
		}
		PreviewSubject::WizardsTower { .. } => {
			if let Some(tower) = cache.wizards_tower.clone() {
				commands
					.spawn_scene((
						tower.scene_with_lod(&lod_ref),
						bsn! {
							template_value(transform)
							Visibility::default()
						},
					))
					.insert(PreviewRoot)
					.insert(tower);
			}
		}
		PreviewSubject::StackedRings { .. } => {
			if let Some(rings) = cache.stacked_rings.as_ref() {
				spawn_preview(
					&mut commands,
					transform,
					ComponentsOnly(rings).scene_with_lod(&lod_ref),
				);
			}
		}
		PreviewSubject::Bedroom { .. } => {
			if let Some(bedroom) = cache.bedroom.as_ref() {
				spawn_preview(
					&mut commands,
					transform,
					ComponentsOnly(bedroom).scene_with_lod(&lod_ref),
				);
			}
		}
	}
}

fn spawn_preview(commands: &mut Commands, transform: Transform, scene: impl bevy::scene::Scene) {
	commands
		.spawn_scene((
			scene,
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.insert(PreviewRoot);
}
