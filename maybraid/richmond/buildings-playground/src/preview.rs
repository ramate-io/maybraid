//! Preview subject sync. Viewer tracking lives in [`lod::LodFinePassPlugin`].

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use lod::gen::LodScene;
use lod::LodViewerState;
use richmond_building_components::partitions::rough_stonework::{
	RoughStonework180, RoughStonework90, RoughStoneworkHeader90, RoughStoneworkLinear,
};
use richmond_building_components::placed::Placement;
use richmond_building_components::roofs::{RoofGeometry, RoofNode};
use richmond_buildings::bedroom::Bedroom;
use richmond_buildings::stacked_rings::StackedRings;
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
	Header90,
	HalfTriangularHip {
		pitch_degrees: f32,
	},
	RectangularHalfGable {
		length_units: u32,
		pitch_degrees: f32,
	},
	WizardsTower { noise: f32 },
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
		Self {
			subject: PreviewSubject::None,
			transform: Transform::IDENTITY,
		}
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		match self.subject {
			PreviewSubject::None => "preview: (none — `/show …`)".into(),
			PreviewSubject::Linear => "preview: rough-stonework linear".into(),
			PreviewSubject::Arc90 => "preview: rough-stonework arc-90".into(),
			PreviewSubject::Arc180 => "preview: rough-stonework arc-180".into(),
			PreviewSubject::Header90 => "preview: rough-stonework header-90".into(),
			PreviewSubject::HalfTriangularHip { pitch_degrees } => {
				format!("preview: half-triangular-hip (pitch={pitch_degrees:.1}°)")
			}
			PreviewSubject::RectangularHalfGable {
				length_units,
				pitch_degrees,
			} => format!(
				"preview: rectangular-half-gable (len={length_units} pitch={pitch_degrees:.1}°)"
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
			PreviewSubject::StackedRings {
				radius,
				floor_count,
				floor_height,
			} => {
				let r = (*radius).max(1e-4);
				let h = (*floor_count as f32) * (*floor_height).max(1e-4);
				Aabb3d::from_min_max(Vec3::new(-r, 0.0, -r), Vec3::new(r, h, r))
			}
			PreviewSubject::WizardsTower { .. } => {
				Aabb3d::from_min_max(Vec3::new(-4.0, 0.0, -4.0), Vec3::new(4.0, 3.0, 4.0))
			}
			PreviewSubject::Bedroom { extent, .. } => Aabb3d::from_min_max(Vec3::ZERO, *extent),
			PreviewSubject::HalfTriangularHip { .. } => {
				Aabb3d::from_min_max(Vec3::new(0.0, -0.2, -1.0), Vec3::new(1.0, 1.0, 0.0))
			}
			PreviewSubject::RectangularHalfGable { length_units, .. } => {
				let len = (*length_units).max(1) as f32;
				Aabb3d::from_min_max(Vec3::new(0.0, -0.2, -1.0), Vec3::new(len, 1.0, 0.0))
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
		match &config.subject {
			PreviewSubject::WizardsTower { noise } => {
				let footprint = CellConstraints::cell_owned(Aabb3d::from_min_max(
					Vec3::new(-4.0, 0.0, -4.0),
					Vec3::new(4.0, 3.0, 4.0),
				));
				self.wizards_tower = Some(WizardsTower::new(&footprint, *noise));
			}
			PreviewSubject::StackedRings {
				floor_count,
				floor_height,
				radius,
			} => {
				self.stacked_rings = Some(StackedRings::new(*floor_count, *floor_height, *radius));
			}
			PreviewSubject::Bedroom {
				extent,
				noise,
				spaciousness,
				occupancy,
				door,
			} => {
				let mut room =
					CellConstraints::cell_owned(Aabb3d::from_min_max(Vec3::ZERO, *extent));
				if *door {
					room.circulation.front = Some(CirculationEntry(vec![(
						Aabb2d {
							min: Vec2::new(0.35, 0.0),
							max: Vec2::new(0.65, 0.9),
						},
						vec![CirculationRequestStatus::Required],
					)]));
				}
				self.bedroom = Some(Bedroom::with_fill(
					room,
					*noise,
					BedroomFillParams {
						spaciousness: *spaciousness,
						occupancy: *occupancy,
					},
				));
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
			spawn_preview(
				&mut commands,
				transform,
				RoughStoneworkLinear.scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Arc90 => {
			spawn_preview(
				&mut commands,
				transform,
				RoughStonework90.scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Arc180 => {
			spawn_preview(
				&mut commands,
				transform,
				RoughStonework180.scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::Header90 => {
			spawn_preview(
				&mut commands,
				transform,
				RoughStoneworkHeader90.scene_with_lod(&lod_ref),
			);
		}
		PreviewSubject::HalfTriangularHip { pitch_degrees } => {
			let roof = RoofNode::shepherds_thatch(
				RoofGeometry::half_triangular_hip(*pitch_degrees),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, roof.scene_with_lod(&lod_ref));
		}
		PreviewSubject::RectangularHalfGable {
			length_units,
			pitch_degrees,
		} => {
			let roof = RoofNode::shepherds_thatch(
				RoofGeometry::rectangular_half_gable(*length_units, *pitch_degrees),
				Placement::IDENTITY,
			);
			spawn_preview(&mut commands, transform, roof.scene_with_lod(&lod_ref));
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
				spawn_preview(&mut commands, transform, rings.scene_with_lod(&lod_ref));
			}
		}
		PreviewSubject::Bedroom { .. } => {
			if let Some(bedroom) = cache.bedroom.as_ref() {
				spawn_preview(&mut commands, transform, bedroom.scene_with_lod(&lod_ref));
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
