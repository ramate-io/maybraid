//! Integration-style tests for dispatch parenting.

use anyhow::Result;
use bevy::prelude::*;

use crate::dispatch::{process_render_dispatches_simple, DispatchRenderItem, RenderItem};

#[derive(Component)]
struct TestCtx;

#[derive(Clone)]
struct TestItem;

impl RenderItem<TestCtx> for TestItem {
	fn spawn_render_items(&self, commands: &mut Commands, dispatch_entity: Entity, _ctx: &TestCtx) {
		commands.entity(dispatch_entity).with_children(|parent| {
			parent.spawn(Name::new("renderit:test_child"));
		});
	}
}

#[test]
fn simple_dispatch_spawns_named_child() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_systems(Update, process_render_dispatches_simple::<TestItem, TestCtx>);
	app.world_mut().spawn((DispatchRenderItem::new(TestItem), TestCtx));
	app.update();
	let mut q = app.world_mut().query_filtered::<&Name, With<Name>>();
	let names: Vec<String> = q.iter(app.world()).map(|n| n.to_string()).collect();
	assert!(
		names.iter().any(|s| s.contains("renderit:test_child")),
		"expected child Name, got {names:?}",
	);
	Ok(())
}
