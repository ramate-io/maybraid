//! Quick e2e check: does `apply_scene` extend or replace `Children`?

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use bevy::app::TaskPoolPlugin;
	use bevy::asset::AssetPlugin;
	use bevy::prelude::*;
	use bevy::scene::{bsn, ScenePlugin};

	#[derive(Component, Default, FromTemplate)]
	struct SceneChild;

	#[derive(Component)]
	struct PreExistingChild;

	fn test_app() -> App {
		let mut app = App::new();
		app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default(), ScenePlugin));
		app
	}

	#[test]
	fn apply_scene_children_behavior() -> Result<()> {
		let mut app = test_app();
		let world = app.world_mut();

		let pre_existing = world.spawn(PreExistingChild).id();
		let root = world.spawn(Name::new("root")).add_child(pre_existing).id();

		assert_eq!(world.entity(root).get::<Children>().map(|c| c.len()), Some(1));

		let scene = bsn! {
			Children [ #SceneChild SceneChild ]
		};
		world.entity_mut(root).apply_scene(scene)?;

		let children: Vec<Entity> = world
			.entity(root)
			.get::<Children>()
			.map(|c| c.iter().collect())
			.unwrap_or_default();

		// Scene child is spawned and linked.
		assert_eq!(children.len(), 1);
		assert!(world.entity(children[0]).contains::<SceneChild>());

		// Pre-existing child entity still exists, but is no longer listed under root.
		assert!(world.get_entity(pre_existing).is_ok());
		assert!(!children.contains(&pre_existing));

		Ok(())
	}
}
