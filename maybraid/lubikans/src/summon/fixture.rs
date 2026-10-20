//! Fixture/example adapter: catalog row plus placeholder capsule presentation.
//!
//! Core summon admission does not touch [`Mesh`] or [`StandardMaterial`]. Tests
//! and the preview example opt in through [`install_fixture_summon`].

use bevy::prelude::*;

use super::{
	fixture_summon_definition, LubikanSummon, LubikanSummonCatalog, LubikanSummonPresentation,
	LubikanSummonRecipe, PLACEHOLDER_SUMMON_CAPSULE_LENGTH, PLACEHOLDER_SUMMON_RADIUS,
};
use crate::catalog::LubikanCatalog;
use crate::LubikanSystems;

/// Register the fixture definition, recipe, and capsule presenter.
pub fn install_fixture_summon(app: &mut App) {
	app.world_mut()
		.resource_mut::<LubikanCatalog>()
		.register(fixture_summon_definition());
	app.world_mut()
		.resource_mut::<LubikanSummonCatalog>()
		.register(LubikanSummonRecipe::fixture());
	app.add_systems(Update, attach_fixture_capsule_meshes.after(LubikanSystems::Summon));
}

fn attach_fixture_capsule_meshes(
	added: Query<(Entity, &LubikanSummon), Added<LubikanSummon>>,
	recipes: Res<LubikanSummonCatalog>,
	meshes: Option<ResMut<Assets<Mesh>>>,
	materials: Option<ResMut<Assets<StandardMaterial>>>,
	mut commands: Commands,
) {
	let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
		return;
	};
	for (entity, live) in &added {
		let Some(recipe) = recipes.get(live.definition) else {
			continue;
		};
		if recipe.presentation != LubikanSummonPresentation::PlaceholderCapsule {
			continue;
		}
		let mesh = meshes
			.add(Capsule3d::new(PLACEHOLDER_SUMMON_RADIUS, PLACEHOLDER_SUMMON_CAPSULE_LENGTH));
		let material = materials.add(StandardMaterial {
			base_color: Color::srgb(0.18, 0.78, 0.74),
			unlit: true,
			..default()
		});
		commands.entity(entity).try_insert((
			Mesh3d(mesh),
			MeshMaterial3d(material),
			Visibility::default(),
		));
	}
}
