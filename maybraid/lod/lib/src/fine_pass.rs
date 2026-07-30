//! Fine-phase LOD: track a viewer transform, update host levels, fulfill lazy roots.
//!
//! Pipeline (no camera types here):
//! [`LodViewer`] transform → [`LodViewerState`] → [`update_lod_host_levels`] →
//! [`crate::sync_lod_level_roots`] → [`fulfill_lod_level_spawn`].
//!
//! Construct [`LodRef`] ephemerally from [`LodViewerState`] + [`LodHostBounds`]
//! (no owned `LodRef` component).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::gen::LodScene;
use crate::lod_level::LodSceneLevel;
use crate::lod_ref::LodRef;
use crate::lod_scene_host::{
	sync_lod_level_roots, LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
};

/// Marker: this entity's [`Transform`] drives fine-phase [`LodRef`] construction.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodViewer;

/// AABB for a [`LodSceneHost`] used when building ephemeral [`LodRef`]s.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodHostBounds(pub Aabb3d);

impl Default for LodHostBounds {
	fn default() -> Self {
		Self(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
	}
}

/// Previous / current viewer pose for ephemeral [`LodRef`]s (no owned component).
#[derive(Resource, Debug)]
pub struct LodViewerState {
	pub entity: Entity,
	pub previous: Transform,
	pub current: Transform,
	/// True when `current.translation` differs from `previous` this frame.
	pub translated: bool,
}

impl Default for LodViewerState {
	fn default() -> Self {
		Self {
			entity: Entity::PLACEHOLDER,
			previous: Transform::IDENTITY,
			current: Transform::IDENTITY,
			translated: false,
		}
	}
}

impl LodViewerState {
	/// Borrowed [`LodRef`] for `bounds` this frame (no clones of transforms).
	pub fn lod_ref<'a>(&'a self, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: self.entity,
			previous_transform: &self.previous,
			current_transform: &self.current,
			bounds,
		}
	}
}

/// System set ordering for the fine pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodFinePassSystems {
	/// Copy [`LodViewer`] transforms into [`LodViewerState`].
	Track,
	/// Write desired [`LodSceneLevel`] on hosts.
	UpdateLevels,
	/// Show/hide roots and enqueue [`LodLevelSpawnRequest`] (`sync_lod_level_roots`).
	SyncRoots,
	/// Spawn missing level-root content for hosts with a spawn request.
	Fulfill,
}

pub(crate) fn configure_fine_pass_sets(app: &mut App) {
	app.configure_sets(
		Update,
		(
			LodFinePassSystems::Track,
			LodFinePassSystems::UpdateLevels,
			LodFinePassSystems::SyncRoots,
			LodFinePassSystems::Fulfill,
		)
			.chain(),
	);
}

/// Copy the primary [`LodViewer`] transform into [`LodViewerState`].
pub fn track_lod_viewer(
	viewers: Query<(Entity, &Transform), With<LodViewer>>,
	mut state: ResMut<LodViewerState>,
) {
	state.translated = false;
	let Ok((entity, transform)) = viewers.single() else {
		return;
	};
	state.previous = state.current;
	state.current = *transform;
	state.entity = entity;
	state.translated =
		(state.previous.translation - state.current.translation).length_squared() > 1e-8;
}

/// Fine-phase: set host [`LodSceneLevel`] from [`LodViewerState`] + [`LodHostBounds`].
pub fn update_lod_host_levels<T: Component + LodScene>(
	viewer: Res<LodViewerState>,
	mut hosts: Query<(&T, &LodHostBounds, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	if viewer.entity == Entity::PLACEHOLDER {
		return;
	}
	for (scene, bounds, mut level) in &mut hosts {
		let lod_ref = viewer.lod_ref(&bounds.0);
		let desired = scene.scene_lod_level(&lod_ref);
		if *level != desired {
			*level = desired;
		}
	}
}

/// Spawn a missing level root under [`LodLevelRoots`], then clear the request.
pub fn fulfill_lod_level_spawn<T: Component + LodScene>(
	mut commands: Commands,
	viewer: Res<LodViewerState>,
	hosts: Query<
		(Entity, &T, &LodHostBounds, &LodLevelSpawnRequest, &Children),
		With<LodSceneHost>,
	>,
	level_roots_heads: Query<(Entity, Option<&Children>), With<LodLevelRoots>>,
) {
	if viewer.entity == Entity::PLACEHOLDER {
		return;
	}

	for (host, scene, bounds, request, host_children) in &hosts {
		let lod_ref = viewer.lod_ref(&bounds.0);

		let mut roots_entity = None;
		for child in host_children.iter() {
			if level_roots_heads.contains(child) {
				roots_entity = Some(child);
				break;
			}
		}

		let Some(roots_entity) = roots_entity else {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		};

		if let Ok((_, Some(root_children))) = level_roots_heads.get(roots_entity) {
			for child in root_children.iter() {
				commands.entity(child).insert(Visibility::Hidden);
			}
		}

		let content: Box<dyn bevy::scene::Scene> =
			Box::new(scene.scene_with_level(&lod_ref, request.level));
		let children = vec![content];
		let level = request.level;
		let level_root = bsn! {
			template_value(LodLevelRoot(level))
			Transform::default()
			Visibility::Inherited
			Children [ {children} ]
		};
		let child = commands.spawn_scene(level_root).id();
		commands.entity(roots_entity).add_child(child);
		commands.entity(host).remove::<LodLevelSpawnRequest>();
	}
}

/// Initializes [`LodViewerState`], tracks [`LodViewer`], and schedules root sync.
///
/// Register per-type update/fulfill with [`add_fine_pass_for`].
pub struct LodFinePassPlugin;

impl Plugin for LodFinePassPlugin {
	fn build(&self, app: &mut App) {
		configure_fine_pass_sets(app);
		app.init_resource::<LodViewerState>().add_systems(
			Update,
			(
				track_lod_viewer.in_set(LodFinePassSystems::Track),
				sync_lod_level_roots.in_set(LodFinePassSystems::SyncRoots),
			),
		);
	}
}

/// Register fine-phase update + fulfill for one [`LodScene`] host component type.
pub fn add_fine_pass_for<T: Component + LodScene>(app: &mut App) {
	configure_fine_pass_sets(app);
	app.add_systems(
		Update,
		(
			update_lod_host_levels::<T>.in_set(LodFinePassSystems::UpdateLevels),
			fulfill_lod_level_spawn::<T>.in_set(LodFinePassSystems::Fulfill),
		),
	);
}
