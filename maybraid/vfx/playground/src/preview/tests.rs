use crate::commands::{LoopEnabled, VfxShowRequest};
use crate::inspector::{inspect_preview, PreviewInspector};
use crate::preview::{PreviewRecipes, PreviewSelection, PreviewStep, VfxPreviewAppExt};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use maybraid_vfx::spawn::VfxPendingLayers;
use maybraid_vfx::{EffectLayer, LightPulse, VfxInstance, VfxLoadFailure};

#[derive(Component, Clone, maybraid_vfx::EffectRecipe)]
#[effect(name = "test-impact", layers = Self::layers)]
struct TestImpact {
	brightness: f32,
}
impl TestImpact {
	fn layers(&self) -> [EffectLayer; 2] {
		let light = EffectLayer::light(LightPulse {
			color: Color::WHITE,
			peak_intensity: self.brightness,
			range: 1.,
			fade: 1.,
		});
		[light.clone().with_name("flash"), light.with_name("echo").with_delay(0.3)]
	}
}

fn app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
		.init_asset::<maybraid_vfx::PackagedFlipbook>()
		.init_asset::<maybraid_vfx::PackagedVolumeFlipbook>()
		.init_resource::<PreviewInspector>()
		.init_resource::<LoopEnabled>()
		.add_message::<VfxLoadFailure>()
		.register_vfx_preview("impact", || TestImpact { brightness: 3. })
		.add_systems(Last, inspect_preview);
	app
}

fn spawn(app: &mut App, selection: PreviewSelection) -> Result<Entity, Box<dyn std::error::Error>> {
	let preview = app
		.world()
		.resource::<PreviewRecipes>()
		.build("impact")
		.ok_or("missing factory")?;
	let request = VfxShowRequest {
		effect: "impact".into(),
		scale: 2.,
		intensity: 1.,
		distance: 4.,
		seed: Some(7),
		playback: 1.,
	};
	let mut inspector = PreviewInspector::default();
	let (root, _) =
		preview.spawn(&mut app.world_mut().commands(), &request, &selection, &mut inspector);
	app.world_mut().flush();
	app.insert_resource(inspector);
	Ok(root)
}

#[test]
fn custom_preview_keeps_typed_recipe_placement_seed_and_authored_indices(
) -> Result<(), Box<dyn std::error::Error>> {
	let mut app = app();
	let root = spawn(&mut app, PreviewSelection { solo: Some(1), ..default() })?;
	assert_eq!(app.world().get::<TestImpact>(root).ok_or("missing recipe")?.brightness, 3.);
	assert_eq!(app.world().get::<VfxInstance>(root).ok_or("missing instance")?.seed, 7);
	assert_eq!(
		app.world().get::<Transform>(root).ok_or("missing placement")?.scale,
		Vec3::splat(2.)
	);
	assert_eq!(app.world().get::<VfxPendingLayers>(root).ok_or("missing pending")?.layers[0].0, 0);
	app.update();
	let rows = &app.world().resource::<PreviewInspector>().layers;
	assert_eq!(rows[0].runtime_index, None);
	assert_eq!(rows[0].status, "muted");
	assert_eq!(rows[1].runtime_index, Some(0));
	assert!(rows[1].status.starts_with("pending"));
	Ok(())
}

#[test]
fn inspector_tracks_loading_playing_finished_and_retains_failure(
) -> Result<(), Box<dyn std::error::Error>> {
	let mut app = app();
	let root = spawn(&mut app, PreviewSelection::default())?;
	app.update();
	assert_eq!(app.world().resource::<PreviewInspector>().layers[0].status, "loading");
	app.world_mut().get_mut::<VfxInstance>(root).ok_or("missing instance")?.armed = true;
	app.update();
	assert!(app.world().resource::<PreviewInspector>().layers[0]
		.status
		.starts_with("playing"));
	app.world_mut().despawn(root);
	app.update();
	assert_eq!(app.world().resource::<PreviewInspector>().layers[0].status, "finished");
	let root = spawn(&mut app, PreviewSelection::default())?;
	app.world_mut().write_message(VfxLoadFailure {
		root,
		effect: "test-impact".into(),
		layer: "echo".into(),
		path: "missing.flipbook.json".into(),
		error: "missing image.ktx2".into(),
	});
	app.world_mut().despawn(root);
	app.update();
	assert!(!app.world().resource::<LoopEnabled>().0);
	assert!(app
		.world()
		.resource::<PreviewInspector>()
		.summary()
		.contains("missing image.ktx2"));
	Ok(())
}

#[test]
fn explicit_step_advances_once_and_stays_paused() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.init_resource::<PreviewStep>()
		.add_systems(First, crate::preview::step_preview.after(bevy::time::TimeSystems));
	app.update();
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	let before = app.world().resource::<Time>().elapsed_secs();
	app.world_mut().resource_mut::<PreviewStep>().0 = Some(std::time::Duration::from_secs_f32(0.1));
	app.update();
	assert!((app.world().resource::<Time>().elapsed_secs() - before - 0.1).abs() < 1e-6);
	assert!(app.world().resource::<Time<Virtual>>().is_paused());
	app.update();
	assert_eq!(app.world().resource::<Time>().delta_secs(), 0.);
}
