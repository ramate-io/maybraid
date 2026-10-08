use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use lod::LodPresentGate;

use crate::{
	subscribe_mode, ActiveGenerationMode, Generate, GenerationMode, GenerationModePlugin, Layer,
	LayerGenerationCore, LayerModeConfig, LayerPresentation, LayerPresentationCore, Present,
	Scheme,
};

struct Alpha;
struct Beta;

impl GenerationMode for Alpha {}
impl GenerationMode for Beta {}

#[derive(Resource, Default, Debug)]
struct StubStore {
	installs: u32,
	presents: u32,
	applies: u32,
	clears: u32,
	seed: u32,
}

struct Ground;

impl Layer for Ground {
	const LABEL: &'static str = "ground";
	type Config = u32;

	fn install_generation(app: &mut App) {
		let mut store = app.world_mut().get_resource_or_insert_with(StubStore::default);
		store.installs += 1;
	}

	fn apply_generation(world: &mut World, config: &Self::Config) {
		let mut store = world.resource_mut::<StubStore>();
		store.seed = *config;
		store.applies += 1;
	}

	fn clear_generation(world: &mut World) {
		world.resource_mut::<StubStore>().clears += 1;
	}

	fn require_lower(_app: &App) {}
}

impl LayerPresentation for Ground {
	fn install_presentation(app: &mut App) {
		let mut store = app.world_mut().get_resource_or_insert_with(StubStore::default);
		store.presents += 1;
	}
}

impl Scheme<Ground> for Alpha {
	fn install(_app: &mut App, _config: &u32) {}
}

impl Scheme<Ground> for Beta {
	fn install(_app: &mut App, _config: &u32) {}
}

struct Silent;

impl Layer for Silent {
	const LABEL: &'static str = "silent";
	type Config = ();

	fn install_generation(_app: &mut App) {}

	fn apply_generation(_world: &mut World, _config: &()) {}

	fn require_lower(_app: &App) {}
}

impl LayerPresentation for Silent {
	fn install_presentation(_app: &mut App) {}
}

impl Scheme<Silent> for Alpha {
	fn install(_app: &mut App, _config: &()) {}
}

struct Clash;

impl Layer for Clash {
	const LABEL: &'static str = "ground";
	type Config = ();

	fn install_generation(_app: &mut App) {}

	fn apply_generation(_world: &mut World, _config: &()) {}

	fn require_lower(_app: &App) {}
}

impl Scheme<Clash> for Alpha {
	fn install(_app: &mut App, _config: &()) {}
}

fn hop(app: &mut App, mode: ActiveGenerationMode) {
	app.world_mut().resource_mut::<NextState<ActiveGenerationMode>>().set(mode);
	app.update();
}

#[test]
fn shared_install_applies_on_enter_and_optional_clear() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<Alpha>::initial(),
		GenerationModePlugin::<Beta>::default(),
		Generate::<Alpha, Ground>::new(1),
		Generate::<Beta, Ground>::new(2),
		Present::<Alpha, Ground>::default(),
	));
	app.finish();
	let store = app.world().resource::<StubStore>();
	anyhow::ensure!(store.installs == 1, "shared generation install ran {}", store.installs);
	anyhow::ensure!(store.presents == 1, "shared presentation install ran {}", store.presents);
	anyhow::ensure!(
		app.is_plugin_added::<LayerGenerationCore<Ground>>(),
		"generation core is installed"
	);
	anyhow::ensure!(
		app.is_plugin_added::<LayerPresentationCore<Ground>>(),
		"presentation core is installed"
	);
	app.update();
	anyhow::ensure!(
		app.world().resource::<StubStore>().seed == 1,
		"initial mode applied {}",
		app.world().resource::<StubStore>().seed
	);
	anyhow::ensure!(app.world().resource::<LodPresentGate<Ground>>().open, "alpha opens");

	hop(&mut app, ActiveGenerationMode::of::<Beta>());
	let store = app.world().resource::<StubStore>();
	anyhow::ensure!(store.seed == 2, "beta applied {}", store.seed);
	anyhow::ensure!(store.clears >= 1, "exit clear ran {}", store.clears);
	anyhow::ensure!(!app.world().resource::<LodPresentGate<Ground>>().open, "beta is unsubscribed");

	hop(&mut app, ActiveGenerationMode::of::<Alpha>());
	anyhow::ensure!(
		app.world().resource::<StubStore>().seed == 1,
		"return restores {}",
		app.world().resource::<StubStore>().seed
	);
	anyhow::ensure!(app.world().resource::<LodPresentGate<Ground>>().open, "alpha opens again");
	Ok(())
}

#[test]
fn plugin_order_does_not_matter() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		Generate::<Beta, Ground>::new(2),
		Generate::<Alpha, Ground>::new(1),
		GenerationModePlugin::<Alpha>::initial(),
		GenerationModePlugin::<Beta>::default(),
	));
	app.finish();
	app.update();
	anyhow::ensure!(
		app.world().resource::<StubStore>().seed == 1,
		"generation before mode still started from alpha: {}",
		app.world().resource::<StubStore>().seed
	);
	anyhow::ensure!(
		app.world().resource::<LayerModeConfig<Alpha, Ground>>().config == 1,
		"alpha keeps its config"
	);
	Ok(())
}

#[test]
fn silent_layer_skips_exit_clear() -> anyhow::Result<()> {
	#[derive(Resource, Default)]
	struct Clears(u32);

	struct NoClear;

	impl Layer for NoClear {
		const LABEL: &'static str = "noclear";
		type Config = ();

		fn install_generation(app: &mut App) {
			app.init_resource::<Clears>();
		}

		fn apply_generation(_world: &mut World, _config: &()) {}

		fn require_lower(_app: &App) {}
	}

	impl Scheme<NoClear> for Alpha {
		fn install(_app: &mut App, _config: &()) {}
	}

	impl Scheme<NoClear> for Beta {
		fn install(_app: &mut App, _config: &()) {}
	}

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<Alpha>::initial(),
		GenerationModePlugin::<Beta>::default(),
		Generate::<Alpha, NoClear>::default(),
		Generate::<Beta, NoClear>::default(),
	));
	app.finish();
	app.update();
	hop(&mut app, ActiveGenerationMode::of::<Beta>());
	anyhow::ensure!(app.world().resource::<Clears>().0 == 0, "default clear is a no-op");
	Ok(())
}

#[test]
fn present_without_generation_names_the_missing_core() {
	let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		Present::<Alpha, Ground>::default().finish(&mut App::new());
	}));
	assert!(failed.is_err(), "present without generate must fail loudly");
}

#[test]
fn duplicate_label_panics_at_build() {
	let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<Alpha>::initial(),
			Generate::<Alpha, Ground>::new(1),
			Generate::<Alpha, Clash>::default(),
		));
	}));
	assert!(failed.is_err(), "a second layer with LABEL ground must fail");
}

#[test]
fn same_layer_may_register_from_generate_and_present() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<Alpha>::initial(),
		Generate::<Alpha, Silent>::default(),
		Present::<Alpha, Silent>::default(),
	));
	app.finish();
	anyhow::ensure!(app.is_plugin_added::<LayerGenerationCore<Silent>>());
	anyhow::ensure!(app.is_plugin_added::<LayerPresentationCore<Silent>>());
	Ok(())
}

#[test]
fn subscribe_mode_is_not_required_when_present_is_added() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		StatesPlugin,
		GenerationModePlugin::<Alpha>::initial(),
		Generate::<Alpha, Ground>::new(1),
		Present::<Alpha, Ground>::default(),
	));
	subscribe_mode::<Ground, Alpha>(&mut app);
	app.finish();
	app.update();
	anyhow::ensure!(app.world().resource::<LodPresentGate<Ground>>().open);
	Ok(())
}
