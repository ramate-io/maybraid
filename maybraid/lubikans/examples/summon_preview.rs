//! Headless-friendly fixture summon preview: `LubikansPlugin` + placeholder capsules.
//!
//! X11 `scrot` of the Vulkan swapchain stays black on xvfb, so this example
//! GPU-readbacks the primary window via Bevy `Screenshot`.
//!
//! ```bash
//! SUMMON_PREVIEW_DIR=/tmp/summon-preview \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   xvfb-run -a cargo run -p lubikans --example summon_preview
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use lubikans::{
	install_fixture_summon, LubikanActivationRequest, LubikanLedger, LubikanOwner,
	LubikanSummonCatalog, LubikanSummonConfig, LubikanSummonDeparture, LubikanSummonRecipe,
	LubikanSummons, LubikansPlugin, PLACEHOLDER_SUMMON_ACTIVATION_COST,
};

fn main() {
	let mut app = App::new();
	app.add_plugins(
		DefaultPlugins
			.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Lubikan summon preview".into(),
					resolution: (640, 360).into(),
					..default()
				}),
				..default()
			})
			.disable::<AudioPlugin>(),
	)
	.add_plugins(LubikansPlugin)
	.insert_resource(LubikanSummonConfig { lifetime_secs: 4.0, ..default() });
	install_fixture_summon(&mut app);
	app.world_mut()
		.resource_mut::<LubikanSummonCatalog>()
		.register(LubikanSummonRecipe { lifetime_secs: 4.0, ..LubikanSummonRecipe::fixture() });
	app.add_systems(Startup, spawn_preview)
		.add_systems(
			Update,
			(fund_and_request, sync_status, capture_preview, quit_after_shots).chain(),
		)
		.run();
}

#[derive(Component)]
struct PreviewStatus;

#[derive(Resource)]
struct PreviewOwner(Entity);

#[derive(Resource, Default)]
struct PreviewPulse(Option<&'static str>);

#[derive(Resource)]
struct PreviewDir(PathBuf);

#[derive(Resource, Default)]
struct PreviewShots {
	warmup: bool,
	spawned: bool,
	departed: bool,
}

fn shot_dir() -> PathBuf {
	std::env::var_os("SUMMON_PREVIEW_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/summon-preview"))
}

fn spawn_preview(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let dir = shot_dir();
	let _ = std::fs::create_dir_all(&dir);
	commands.insert_resource(PreviewDir(dir));
	commands.init_resource::<PreviewPulse>();
	commands.init_resource::<PreviewShots>();
	commands.insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)));
	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(6.2, 2.6, -1.0).looking_at(Vec3::new(0.0, 0.8, -1.0), Vec3::Y),
	));
	commands.spawn((
		DirectionalLight { illuminance: 12_000.0, shadow_maps_enabled: false, ..default() },
		Transform::from_xyz(2.0, 6.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::new(8.0, 8.0)));
	let floor_mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.12, 0.13, 0.15),
		unlit: true,
		..default()
	});
	commands.spawn((Mesh3d(floor), MeshMaterial3d(floor_mat), Transform::from_xyz(0.0, 0.0, 0.0)));
	let owner_mesh = meshes.add(Capsule3d::new(0.35, 0.90));
	let owner_mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.62, 0.58, 0.50),
		unlit: true,
		..default()
	});
	let owner = commands
		.spawn((
			LubikanOwner,
			Transform::from_xyz(0.0, 0.8, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y),
			Mesh3d(owner_mesh),
			MeshMaterial3d(owner_mat),
			Name::new("summon-preview-owner"),
		))
		.id();
	commands.insert_resource(PreviewOwner(owner));
	commands.spawn((
		PreviewStatus,
		Text::new("Fixture summon"),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(16.0),
			left: Val::Px(16.0),
			..default()
		},
		TextFont { font_size: FontSize::Px(22.0), ..default() },
		TextColor(Color::srgb(0.92, 0.86, 0.55)),
	));
}

fn fund_and_request(
	mut ledgers: Query<&mut LubikanLedger>,
	owner: Option<Res<PreviewOwner>>,
	mut requests: MessageWriter<LubikanActivationRequest>,
	mut primed: Local<bool>,
) {
	if *primed {
		return;
	}
	let Some(owner) = owner else {
		return;
	};
	let Ok(mut ledger) = ledgers.get_mut(owner.0) else {
		return;
	};
	let definition = lubikans::fixture_summon_definition();
	let account = ledger.ensure_owned(definition);
	if !ledger.select(Some(account)) {
		return;
	}
	ledger.apply_to(account, PLACEHOLDER_SUMMON_ACTIVATION_COST);
	requests.write(LubikanActivationRequest::for_account(owner.0, account));
	*primed = true;
}

fn sync_status(
	summons: Res<LubikanSummons>,
	mut departed: MessageReader<LubikanSummonDeparture>,
	mut pulse: ResMut<PreviewPulse>,
	mut status: Query<&mut Text, With<PreviewStatus>>,
) {
	if departed.read().next().is_some() {
		pulse.0 = Some("departed -> idle");
	}
	if let Ok(mut text) = status.single_mut() {
		*text = Text::new(format!(
			"Fixture summon  live={live}  {pulse}",
			live = summons.live,
			pulse = pulse.0.unwrap_or(if summons.live > 0 { "spawned" } else { "waiting" })
		));
	}
}

fn capture_preview(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<PreviewDir>,
	summons: Res<LubikanSummons>,
	pulse: Res<PreviewPulse>,
	mut shots: ResMut<PreviewShots>,
) {
	if !shots.warmup && time.elapsed_secs() >= 1.0 {
		request_shot(&mut commands, dir.0.join("00-warmup.png"));
		shots.warmup = true;
	}
	if !shots.spawned
		&& shots.warmup
		&& time.elapsed_secs() >= 2.0
		&& summons.live == 1
		&& dir.0.join("00-warmup.png").is_file()
	{
		request_shot(&mut commands, dir.0.join("01-spawned.png"));
		shots.spawned = true;
		info!("preview shot spawned");
	}
	if !shots.departed && pulse.0 == Some("departed -> idle") && time.elapsed_secs() >= 4.2 {
		request_shot(&mut commands, dir.0.join("02-departed.png"));
		shots.departed = true;
		info!("preview shot departed");
	}
}

fn request_shot(commands: &mut Commands, path: impl AsRef<Path>) {
	let path = path.as_ref().to_owned();
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn quit_after_shots(
	time: Res<Time>,
	shots: Res<PreviewShots>,
	dir: Res<PreviewDir>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
) {
	let files_ready = shots.spawned
		&& shots.departed
		&& dir.0.join("01-spawned.png").is_file()
		&& dir.0.join("02-departed.png").is_file();
	if files_ready && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| time.elapsed() > Duration::from_secs(20)
	{
		exit.write(AppExit::Success);
	}
}
