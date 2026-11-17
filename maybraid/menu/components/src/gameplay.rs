//! Play HUD: a shared parent layer plus a tag so pause and overlays hide it
//! the same way.

use bevy::prelude::*;

/// Stamp on gameplay HUD roots that should hide in menus and under focus overlay.
///
/// Unparented roots are adopted onto [`GameplayHudLayer`]. Children that need
/// to yield to the skill-map overlay keep this tag; the map HUD itself is
/// excluded from that hide so the overlay can stay up.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GameplayHud;

/// Full-screen parent of play HUD. Hidden while [`GameplayHudVisible`] is off.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GameplayHudLayer;

/// When `false`, the play HUD layer stays hidden (pause / menu overlays).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameplayHudVisible(pub bool);

impl Default for GameplayHudVisible {
	fn default() -> Self {
		Self(true)
	}
}

/// Play HUD "on" visibility. [`Visibility::Visible`] punches through a hidden
/// [`GameplayHudLayer`] (menus, map sheet), so roots use Inherited instead.
pub fn gameplay_hud_visibility(shown: bool) -> Visibility {
	if shown {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	}
}

pub fn configure_gameplay_hud(app: &mut App) {
	app.init_resource::<GameplayHudVisible>()
		.add_systems(Startup, spawn_gameplay_hud_layer)
		.add_systems(PostStartup, adopt_gameplay_hud)
		.add_systems(Update, (adopt_gameplay_hud, sync_gameplay_hud_layer).chain());
}

fn spawn_gameplay_hud_layer(
	mut commands: Commands,
	existing: Query<Entity, With<GameplayHudLayer>>,
) {
	if !existing.is_empty() {
		return;
	}
	commands.spawn((
		Name::new("gameplay-hud-layer"),
		GameplayHudLayer,
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			..default()
		},
		Visibility::Inherited,
		Pickable::IGNORE,
	));
}

fn adopt_gameplay_hud(
	mut commands: Commands,
	layer: Query<Entity, With<GameplayHudLayer>>,
	orphans: Query<Entity, (With<GameplayHud>, Without<ChildOf>, Without<GameplayHudLayer>)>,
) {
	let Ok(layer) = layer.single() else {
		return;
	};
	for entity in &orphans {
		commands.entity(layer).add_child(entity);
	}
}

fn sync_gameplay_hud_layer(
	visible: Res<GameplayHudVisible>,
	mut layers: Query<&mut Visibility, With<GameplayHudLayer>>,
) {
	let next = gameplay_hud_visibility(visible.0);
	for mut visibility in &mut layers {
		if *visibility != next {
			*visibility = next;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn hud_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		configure_gameplay_hud(&mut app);
		app
	}

	#[test]
	fn layer_hides_inherited_children_when_menus_are_up() {
		let mut app = hud_app();
		let child = app.world_mut().spawn((GameplayHud, Visibility::Inherited)).id();
		app.update();
		let layer = app
			.world_mut()
			.query_filtered::<Entity, With<GameplayHudLayer>>()
			.single(app.world())
			.expect("layer");
		assert_eq!(app.world().entity(child).get::<ChildOf>().map(ChildOf::parent), Some(layer));
		app.world_mut().resource_mut::<GameplayHudVisible>().0 = false;
		app.update();
		assert_eq!(*app.world().get::<Visibility>(layer).expect("vis"), Visibility::Hidden);
		app.world_mut().resource_mut::<GameplayHudVisible>().0 = true;
		app.update();
		assert_eq!(*app.world().get::<Visibility>(layer).expect("vis"), Visibility::Inherited);
	}

	#[test]
	fn play_hud_uses_inherited_so_the_layer_can_hide_it() {
		assert_eq!(gameplay_hud_visibility(true), Visibility::Inherited);
		assert_eq!(gameplay_hud_visibility(false), Visibility::Hidden);
	}

	#[test]
	fn nested_hud_is_not_reparented() {
		let mut app = hud_app();
		app.update();
		let layer = app
			.world_mut()
			.query_filtered::<Entity, With<GameplayHudLayer>>()
			.single(app.world())
			.expect("layer");
		let parent = app.world_mut().spawn((GameplayHud, Visibility::Inherited)).id();
		let nested = app.world_mut().spawn((GameplayHud, ChildOf(parent))).id();
		app.update();
		assert_eq!(app.world().entity(parent).get::<ChildOf>().map(ChildOf::parent), Some(layer));
		assert_eq!(app.world().entity(nested).get::<ChildOf>().map(ChildOf::parent), Some(parent));
	}
}
