//! Preview subject sync: despawn previous root and spawn the requested scene.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use bevy_math::bounding::Aabb3d;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use richmond_building_components::partitions::rough_stonework::{
	RoughStonework180, RoughStonework90, RoughStoneworkHeader90, RoughStoneworkLinear,
};
use richmond_buildings::wizards_tower::WizardsTower;
use richmond_buildings::CellConstraints;

#[derive(Component)]
pub struct PreviewRoot;

#[derive(Clone, Debug, PartialEq)]
pub enum PreviewSubject {
	None,
	Linear,
	Arc90,
	Arc180,
	Header90,
	WizardsTower { noise: f32 },
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
			PreviewSubject::WizardsTower { noise } => {
				format!("preview: wizards-tower (noise={noise:.2})")
			}
		}
	}
}

#[derive(Resource, Default)]
pub(crate) struct PreviewSyncState {
	last: Option<(PreviewSubject, Transform)>,
}

pub fn sync_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut state: Local<PreviewSyncState>,
	roots: Query<Entity, With<PreviewRoot>>,
) {
	let key = (config.subject.clone(), config.transform);
	if state.last.as_ref() == Some(&key) {
		return;
	}
	state.last = Some(key);

	for entity in &roots {
		commands.entity(entity).despawn();
	}

	if matches!(config.subject, PreviewSubject::None) {
		return;
	}

	let identity = Transform::IDENTITY;
	let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};

	let transform = config.transform;
	match &config.subject {
		PreviewSubject::None => {}
		PreviewSubject::Linear => {
			let scene = RoughStoneworkLinear.scene_with_lod(&lod_ref);
			commands
				.spawn_scene(bsn! {
					template_value(transform)
					Children [ ({scene}) ]
				})
				.insert(PreviewRoot);
		}
		PreviewSubject::Arc90 => {
			let scene = RoughStonework90.scene_with_lod(&lod_ref);
			commands
				.spawn_scene(bsn! {
					template_value(transform)
					Children [ ({scene}) ]
				})
				.insert(PreviewRoot);
		}
		PreviewSubject::Arc180 => {
			let scene = RoughStonework180.scene_with_lod(&lod_ref);
			commands
				.spawn_scene(bsn! {
					template_value(transform)
					Children [ ({scene}) ]
				})
				.insert(PreviewRoot);
		}
		PreviewSubject::Header90 => {
			let scene = RoughStoneworkHeader90.scene_with_lod(&lod_ref);
			commands
				.spawn_scene(bsn! {
					template_value(transform)
					Children [ ({scene}) ]
				})
				.insert(PreviewRoot);
		}
		PreviewSubject::WizardsTower { noise } => {
			let footprint = CellConstraints::cell_owned(Aabb3d::from_min_max(
				Vec3::new(-4.0, 0.0, -4.0),
				Vec3::new(4.0, 24.0, 4.0),
			));
			let tower = WizardsTower::new(&footprint, *noise);
			let scene = tower.scene_with_lod(&lod_ref);
			commands
				.spawn_scene(bsn! {
					template_value(transform)
					Children [ ({scene}) ]
				})
				.insert(PreviewRoot);
		}
	}
}
