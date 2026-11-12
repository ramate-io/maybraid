//! Full-screen stained-glass overlay driven by the skill-map steer hold.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui_render::prelude::{UiMaterial, UiMaterialPlugin};

use crate::map::SkillKind;
use crate::tile_material::SkillMapTileMaterial;
use crate::user::{SkillMapHeld, SkillMapMember, SkillMapUser};
use crate::viewport::{SkillMapFirearmHud, SkillMapHud, SkillMapViewport, SkillMapViewportCamera};
use crate::SkillMapEnabled;

const OVERLAY_SHADER: &str =
	concat!("embedded://", env!("CARGO_CRATE_NAME"), "/overlay_material.wesl");

/// Corner inset copied from the viewport chrome so collapsed layout stays put.
const VIEWPORT_PX: f32 = crate::viewport::VIEWPORT_PX;
const VIEWPORT_INSET: f32 = 16.0;
const FRAME_BORDER: f32 = 2.0;
const FRAME_RADIUS: f32 = 14.0;
const FRAME_INNER_RADIUS: f32 = FRAME_RADIUS - FRAME_BORDER;
const FIREARM_CARD_PAD_X: f32 = 8.0;
const FIREARM_CARD_PAD_Y: f32 = 6.0;
const FIREARM_CARD_BORDER: f32 = 1.5;
const FIREARM_CARD_RADIUS: f32 = 8.0;

/// How strongly overlay mode warps the map (and the world seen through it).
/// Was `0.85`; left at `0.28` so refraction stays a hint, not opacity.
pub const OVERLAY_WARP: f32 = 0.28;
/// How much stained-glass *color* washes the warped map. Not coverage.
/// Was `0.08`; ~30% lighter so mosaic stain stays a tint.
pub const OVERLAY_GLASS: f32 = 0.056;
/// Blend of tile shading into overlay (translucent) mode when expanded.
/// This is **not** a veil opacity: `0` keeps gameplay-tile alpha `1`, `1`
/// mixes toward the see-through overlay alpha. Keep in sync with `tile_material.wesl`.
pub const OVERLAY_TILE_AMOUNT: f32 = 1.0;
/// Map-camera clear alpha while expanded. Empty texels only; feed mosaic fills the RT.
/// Was `0.028`.
pub const OVERLAY_CLEAR_ALPHA: f32 = 0.020;
/// Expanded frame fill alpha (collapsed chrome still uses a fixed 0.12). Was `0.012`.
pub const OVERLAY_FRAME_FILL_ALPHA: f32 = 0.008;
/// Keep in sync with `overlay_material.wesl`.
pub const OVERLAY_COMPOSITOR_ALPHA_SAMPLE: f32 = 0.88;
pub const OVERLAY_COMPOSITOR_ALPHA_BIAS: f32 = 0.045;
pub const OVERLAY_COMPOSITOR_CAME_ALPHA: f32 = 0.07;
pub const OVERLAY_MARK_ALPHA_LO: f32 = 0.60;
pub const OVERLAY_MARK_ALPHA_HI: f32 = 0.90;
/// Minimum composited alpha for arrows, power tiles, and the cursor.
pub const OVERLAY_MARK_ALPHA_MIN: f32 = 0.85;
/// Tile overlay amount on the 9f22704 build. Used as a coverage regression bound.
#[cfg(test)]
const OVERLAY_TILE_AMOUNT_NINE_F227: f32 = 0.50;

/// Collapsed corner HUD versus the full-screen stained-glass overlay.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillMapOverlay {
	#[default]
	Collapsed,
	Expanded,
}

impl SkillKind {
	pub fn overlay_clear(self, overlay: SkillMapOverlay) -> Color {
		let base = self.viewport_clear();
		if overlay.expanded() {
			base.with_alpha(OVERLAY_CLEAR_ALPHA)
		} else {
			base
		}
	}
}

impl SkillMapOverlay {
	pub fn from_held(held: bool) -> Self {
		if held {
			Self::Expanded
		} else {
			Self::Collapsed
		}
	}

	pub const fn expanded(self) -> bool {
		matches!(self, Self::Expanded)
	}

	pub const fn amount(self) -> f32 {
		match self {
			Self::Collapsed => 0.0,
			Self::Expanded => 1.0,
		}
	}

	/// Bottom-right corner when inactive; the whole window when held.
	pub fn hud_node(self) -> Node {
		match self {
			Self::Collapsed => Node {
				position_type: PositionType::Absolute,
				bottom: Val::Px(VIEWPORT_INSET),
				right: Val::Px(VIEWPORT_INSET),
				width: Val::Px(VIEWPORT_PX),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Stretch,
				row_gap: Val::Px(8.0),
				..default()
			},
			Self::Expanded => Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				right: Val::Px(0.0),
				top: Val::Px(0.0),
				bottom: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Stretch,
				row_gap: Val::Px(0.0),
				..default()
			},
		}
	}

	pub fn map_frame_node(self) -> Node {
		match self {
			Self::Collapsed => Node {
				width: Val::Px(VIEWPORT_PX),
				height: Val::Px(VIEWPORT_PX),
				border: UiRect::all(Val::Px(FRAME_BORDER)),
				border_radius: BorderRadius::all(Val::Px(FRAME_RADIUS)),
				overflow: Overflow::clip(),
				flex_grow: 0.0,
				..default()
			},
			Self::Expanded => Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				border: UiRect::all(Val::Px(0.0)),
				border_radius: BorderRadius::ZERO,
				overflow: Overflow::clip(),
				flex_grow: 1.0,
				..default()
			},
		}
	}

	pub fn pane_node(self) -> Node {
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			right: Val::Px(0.0),
			top: Val::Px(0.0),
			bottom: Val::Px(0.0),
			border_radius: BorderRadius::all(Val::Px(self.inner_radius())),
			display: if self.expanded() { Display::Flex } else { Display::None },
			..default()
		}
	}

	pub fn firearm_node(self) -> Node {
		let card = Node {
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			row_gap: Val::Px(6.0),
			padding: UiRect::axes(Val::Px(FIREARM_CARD_PAD_X), Val::Px(FIREARM_CARD_PAD_Y)),
			border: UiRect::all(Val::Px(FIREARM_CARD_BORDER)),
			border_radius: BorderRadius::all(Val::Px(FIREARM_CARD_RADIUS)),
			..default()
		};
		match self {
			Self::Collapsed => Node { width: Val::Percent(100.0), ..card },
			Self::Expanded => Node {
				position_type: PositionType::Absolute,
				right: Val::Px(VIEWPORT_INSET),
				bottom: Val::Px(VIEWPORT_INSET),
				width: Val::Px(VIEWPORT_PX),
				..card
			},
		}
	}

	pub fn inner_radius(self) -> f32 {
		if self.expanded() {
			0.0
		} else {
			FRAME_INNER_RADIUS
		}
	}

	pub fn frame_border(self) -> Color {
		if self.expanded() {
			Color::NONE
		} else {
			Color::srgba(1.0, 0.86, 0.22, 0.42)
		}
	}

	pub fn frame_fill(self) -> Color {
		if self.expanded() {
			Color::srgba(0.02, 0.01, 0.04, OVERLAY_FRAME_FILL_ALPHA)
		} else {
			Color::srgba(0.0, 0.0, 0.0, 0.12)
		}
	}

	pub fn live_fill(self) -> Color {
		if self.expanded() {
			Color::NONE
		} else {
			Color::srgba(0.0, 0.0, 0.0, 0.0)
		}
	}

	pub fn tile_amount(self) -> f32 {
		if self.expanded() {
			OVERLAY_TILE_AMOUNT
		} else {
			0.0
		}
	}

	/// Tile overlay α then compositor α for a typical pane (no came, mid noise).
	pub fn pane_alpha(self, tile_base_alpha: f32) -> f32 {
		self.pane_alpha_with(tile_base_alpha, 0.0, 0.5)
	}

	pub fn pane_alpha_with(self, tile_base_alpha: f32, came: f32, noise: f32) -> f32 {
		let sample = SkillMapTileMaterial::overlay_alpha(tile_base_alpha, self.tile_amount());
		OverlayParams::from_overlay(self).composited_alpha(sample, came, noise)
	}

	/// Arrows, power tiles, and the cursor after the compositor (full map-RT alpha).
	pub fn mark_alpha(self, sample_alpha: f32) -> f32 {
		OverlayParams::from_overlay(self).composited_alpha(sample_alpha, 0.0, 0.5)
	}

	pub fn firearm_z(self) -> ZIndex {
		if self.expanded() {
			ZIndex(2)
		} else {
			ZIndex(0)
		}
	}
}

/// Expand-edge cue so overlay SFX fires once per RB hold, not every frame.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillMapOverlayCue {
	pub user: Entity,
}

/// Viewport camera image shown through the overlay compositor.
#[derive(Component, Clone, Debug)]
pub struct SkillMapOverlayPane {
	pub material: Handle<SkillMapOverlayMaterial>,
}

/// Authored kind on the map camera, so overlay can swap the clear color.
#[derive(Component, Clone, Copy, Debug)]
pub struct SkillMapBackdrop {
	pub kind: SkillKind,
}

/// Parameters for the stained-glass compositor. `w` is unused padding.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct OverlayParams {
	pub amount: f32,
	pub warp: f32,
	pub glass: f32,
	pub _pad: f32,
}

impl OverlayParams {
	pub fn collapsed() -> Self {
		Self { amount: 0.0, warp: OVERLAY_WARP, glass: OVERLAY_GLASS, _pad: 0.0 }
	}

	pub fn from_overlay(overlay: SkillMapOverlay) -> Self {
		Self { amount: overlay.amount(), warp: OVERLAY_WARP, glass: OVERLAY_GLASS, _pad: 0.0 }
	}

	/// CPU mirror of `overlay_material.wesl` fragment alpha (`lead = 0.22 + 0.18 * noise`).
	/// High sample alpha (arrows / marks) is kept; low alpha is the land veil.
	pub fn composited_alpha(self, sample_alpha: f32, came: f32, noise: f32) -> f32 {
		let amount = self.amount.clamp(0.0, 1.0);
		let lead = 0.22 + 0.18 * noise;
		let veil = sample_alpha * OVERLAY_COMPOSITOR_ALPHA_SAMPLE
			+ OVERLAY_COMPOSITOR_ALPHA_BIAS
			+ came * lead * OVERLAY_COMPOSITOR_CAME_ALPHA;
		let treated = veil * (1.0 - Self::mark_keep(sample_alpha))
			+ sample_alpha * Self::mark_keep(sample_alpha);
		sample_alpha * (1.0 - amount) + treated * amount
	}

	pub fn mark_keep(sample_alpha: f32) -> f32 {
		let span = OVERLAY_MARK_ALPHA_HI - OVERLAY_MARK_ALPHA_LO;
		let t = ((sample_alpha - OVERLAY_MARK_ALPHA_LO) / span).clamp(0.0, 1.0);
		t * t * (3.0 - 2.0 * t)
	}
}

/// Full-screen compositor: warps the map render target like stained glass.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkillMapOverlayMaterial {
	#[texture(0)]
	#[sampler(1)]
	pub map: Handle<Image>,
	#[uniform(2)]
	pub params: OverlayParams,
}

impl SkillMapOverlayMaterial {
	pub fn new(map: Handle<Image>) -> Self {
		Self { map, params: OverlayParams::collapsed() }
	}

	pub fn set_overlay(&mut self, overlay: SkillMapOverlay) {
		self.params = OverlayParams::from_overlay(overlay);
	}
}

impl UiMaterial for SkillMapOverlayMaterial {
	fn fragment_shader() -> ShaderRef {
		OVERLAY_SHADER.into()
	}
}

pub struct SkillMapOverlayPlugin;

impl Plugin for SkillMapOverlayPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "overlay_material.wesl");
		app.add_plugins(UiMaterialPlugin::<SkillMapOverlayMaterial>::default())
			.add_message::<SkillMapOverlayCue>();
		crate::sound::configure_overlay_audio(app);
	}
}

/// Mirror [`SkillMapHeld`] onto overlay layout, tile overlay uniforms, and the compositor.
pub fn sync_skill_map_overlay(
	enabled: Res<SkillMapEnabled>,
	mut users: Query<(Entity, &SkillMapUser, &SkillMapHeld, &mut SkillMapOverlay)>,
	mut cues: MessageWriter<SkillMapOverlayCue>,
	mut huds: Query<(&SkillMapMember, &mut Node), With<SkillMapHud>>,
	mut frames: Query<
		(&SkillMapMember, &mut Node, &mut BorderColor, &mut BackgroundColor),
		(With<SkillMapViewport>, Without<SkillMapHud>, Without<SkillMapFirearmHud>),
	>,
	mut panes: Query<
		(&SkillMapMember, &SkillMapOverlayPane, &mut Node),
		(Without<SkillMapHud>, Without<SkillMapViewport>, Without<SkillMapFirearmHud>),
	>,
	mut firearms: Query<
		(&SkillMapMember, &mut Node, &mut ZIndex),
		(With<SkillMapFirearmHud>, Without<SkillMapHud>, Without<SkillMapViewport>),
	>,
	mut cameras: Query<
		(&SkillMapMember, &SkillMapBackdrop, &mut Camera),
		(With<SkillMapViewportCamera>, Without<SkillMapHud>),
	>,
	mut overlay_materials: Option<ResMut<Assets<SkillMapOverlayMaterial>>>,
	mut tiles: Option<ResMut<Assets<SkillMapTileMaterial>>>,
) {
	for (entity, user, held, mut overlay) in &mut users {
		let next =
			if enabled.0 { SkillMapOverlay::from_held(held.0) } else { SkillMapOverlay::Collapsed };
		if *overlay != next {
			if next.expanded() {
				cues.write(SkillMapOverlayCue { user: entity });
			}
			*overlay = next;
		}
		apply_overlay_layout(
			user.maps,
			next,
			&mut huds,
			&mut frames,
			&mut panes,
			&mut firearms,
			&mut cameras,
			overlay_materials.as_deref_mut(),
		);
	}
	if let Some(tiles) = tiles.as_deref_mut() {
		let amount = if users.iter().any(|(_, _, _, overlay)| overlay.expanded()) {
			OVERLAY_TILE_AMOUNT
		} else {
			0.0
		};
		set_tile_overlay_amount(tiles, amount);
	}
}

fn apply_overlay_layout(
	session: Entity,
	overlay: SkillMapOverlay,
	huds: &mut Query<(&SkillMapMember, &mut Node), With<SkillMapHud>>,
	frames: &mut Query<
		(&SkillMapMember, &mut Node, &mut BorderColor, &mut BackgroundColor),
		(With<SkillMapViewport>, Without<SkillMapHud>, Without<SkillMapFirearmHud>),
	>,
	panes: &mut Query<
		(&SkillMapMember, &SkillMapOverlayPane, &mut Node),
		(Without<SkillMapHud>, Without<SkillMapViewport>, Without<SkillMapFirearmHud>),
	>,
	firearms: &mut Query<
		(&SkillMapMember, &mut Node, &mut ZIndex),
		(With<SkillMapFirearmHud>, Without<SkillMapHud>, Without<SkillMapViewport>),
	>,
	cameras: &mut Query<
		(&SkillMapMember, &SkillMapBackdrop, &mut Camera),
		(With<SkillMapViewportCamera>, Without<SkillMapHud>),
	>,
	mut overlay_materials: Option<&mut Assets<SkillMapOverlayMaterial>>,
) {
	for (member, mut node) in huds.iter_mut() {
		if member.session == session {
			*node = overlay.hud_node();
		}
	}
	for (member, mut node, mut border, mut background) in frames.iter_mut() {
		if member.session != session {
			continue;
		}
		*node = overlay.map_frame_node();
		*border = BorderColor::all(overlay.frame_border());
		background.0 = if overlay.expanded() { overlay.live_fill() } else { overlay.frame_fill() };
	}
	for (member, pane, mut node) in panes.iter_mut() {
		if member.session != session {
			continue;
		}
		*node = overlay.pane_node();
		if let Some(materials) = overlay_materials.as_mut() {
			if let Some(mut material) = materials.get_mut(&pane.material) {
				material.set_overlay(overlay);
			}
		}
	}
	for (member, mut node, mut z) in firearms.iter_mut() {
		if member.session == session {
			*node = overlay.firearm_node();
			*z = overlay.firearm_z();
		}
	}
	for (member, backdrop, mut camera) in cameras.iter_mut() {
		if member.session != session {
			continue;
		}
		camera.clear_color =
			bevy::camera::ClearColorConfig::Custom(backdrop.kind.overlay_clear(overlay));
	}
}

fn set_tile_overlay_amount(tiles: &mut Assets<SkillMapTileMaterial>, amount: f32) {
	for (_, material) in tiles.iter_mut() {
		material.set_overlay_amount(amount);
	}
}

pub fn spawn_overlay_pane(
	parent: &mut ChildSpawnerCommands,
	materials: &mut Assets<SkillMapOverlayMaterial>,
	image: Handle<Image>,
	member: SkillMapMember,
) {
	let material = materials.add(SkillMapOverlayMaterial::new(image));
	parent.spawn((
		Name::new("skill-map-overlay-pane"),
		SkillMapOverlayPane { material: material.clone() },
		member,
		MaterialNode(material),
		SkillMapOverlay::Collapsed.pane_node(),
		Pickable::IGNORE,
	));
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::feed_view::FeedArrow;
	use crate::tunnel_vision::{
		apply_tunnel_vision_material, hide_gameplay_hud, sync_tunnel_vision_target,
		tick_tunnel_vision, SkillMapTunnelVision, SkillMapTunnelVisionMaterial,
	};
	use crate::user::spawn_skill_maps;
	use crate::viewport::SkillMapCollapsedChrome;
	use crate::SkillMapEnabled;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::time::TimeUpdateStrategy;
	use maybraid_character_controller::{
		register_character_controller, ActorTarget, CharacterControlSystems, OrdinaryLocomotion,
	};
	use maybraid_controllers_core::InputSource;
	use maybraid_virtual_pad_controller::{
		PadButton, PadDevices, VirtualPad, VirtualPadController, VirtualPadEvent,
	};
	use menu_components::GameplayHud;

	#[test]
	fn overlay_layout_fills_the_window_only_when_expanded() {
		let collapsed = SkillMapOverlay::Collapsed.hud_node();
		assert_eq!(collapsed.bottom, Val::Px(VIEWPORT_INSET));
		assert_eq!(collapsed.right, Val::Px(VIEWPORT_INSET));
		assert_eq!(collapsed.width, Val::Px(VIEWPORT_PX));
		let expanded = SkillMapOverlay::Expanded.hud_node();
		assert_eq!(expanded.left, Val::Px(0.0));
		assert_eq!(expanded.top, Val::Px(0.0));
		assert_eq!(expanded.width, Val::Percent(100.0));
		assert_eq!(expanded.height, Val::Percent(100.0));
		assert_eq!(SkillMapOverlay::Expanded.map_frame_node().width, Val::Percent(100.0));
		assert_eq!(SkillMapOverlay::Collapsed.pane_node().display, Display::None);
		assert_eq!(SkillMapOverlay::Expanded.pane_node().display, Display::Flex);
	}

	#[test]
	fn shader_ref_matches_embedded_asset_path() {
		let path = bevy::asset::embedded_path!("overlay_material.wesl");
		let registered = path.to_str().unwrap().replace('\\', "/");
		assert_eq!(
			registered,
			format!("{}/overlay_material.wesl", env!("CARGO_CRATE_NAME")),
			"UiMaterial ShaderRef must use this path; Bevy embeds subdirectory WGSL under src/"
		);
		assert!(OVERLAY_SHADER.ends_with(&registered));
	}

	fn overlay_app() -> App {
		let mut app = App::new();
		register_character_controller(&mut app);
		app.add_plugins(MinimalPlugins)
			.init_resource::<SkillMapEnabled>()
			.init_resource::<SkillMapTunnelVision>()
			.add_message::<SkillMapOverlayCue>()
			.add_systems(
				Update,
				(
					crate::cursor::sync_skill_map_held,
					crate::feed::start_feeds_on_steer_hold,
					crate::feed::tick_feeds,
					sync_skill_map_overlay,
					sync_tunnel_vision_target,
				)
					.chain()
					.after(CharacterControlSystems),
			)
			.add_systems(
				PostUpdate,
				(
					tick_tunnel_vision,
					hide_gameplay_hud,
					apply_tunnel_vision_material,
					crate::sound::play_overlay_activate_sounds,
				)
					.chain(),
			);
		app
	}

	fn tunnel_app() -> App {
		let mut app = overlay_app();
		app.insert_resource(TimeUpdateStrategy::ManualDuration(
			std::time::Duration::from_secs_f32(crate::TUNNEL_VISION_FADE_SECS),
		));
		app
	}

	fn overlay_sound_app() -> App {
		use maybraid_audio::{decode_wav_mono, AudioClip};
		let mut app = overlay_app();
		app.add_plugins(AssetPlugin::default());
		app.init_asset::<AudioClip>();
		let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../assets")
			.join(crate::OVERLAY_ACTIVATE_SOUND);
		let bytes = std::fs::read(&file).unwrap_or_else(|_| panic!("{}", file.display()));
		let clip = AudioClip { frames: decode_wav_mono(&bytes).expect("mono wav") };
		let handle = app.world_mut().resource_mut::<Assets<AudioClip>>().add(clip);
		app.world_mut().insert_resource(crate::SkillMapOverlaySounds::from_clip(handle));
		app
	}

	fn activations(app: &App) -> u32 {
		app.world().resource::<crate::SkillMapOverlaySounds>().activations()
	}

	fn spawn_overlay_user(app: &mut App, ordinary: bool) -> (Entity, Entity) {
		let user = app.world_mut().spawn(OrdinaryLocomotion(ordinary)).id();
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				spawn_skill_maps(&mut commands, user);
			})
			.expect("spawn maps");
		app.world_mut().flush();
		let session = app.world().get::<SkillMapUser>(user).expect("user").maps;
		app.world_mut().entity_mut(session).insert(crate::feed::FeedState::default());
		let member = SkillMapMember { user, session };
		let hud = app
			.world_mut()
			.spawn((SkillMapHud, member, SkillMapOverlay::Collapsed.hud_node()))
			.id();
		app.world_mut().spawn((
			SkillMapViewport,
			member,
			SkillMapOverlay::Collapsed.map_frame_node(),
			BorderColor::all(SkillMapOverlay::Collapsed.frame_border()),
			BackgroundColor(SkillMapOverlay::Collapsed.frame_fill()),
		));
		(user, hud)
	}

	fn assign_pad(app: &mut App, user: Entity) -> Entity {
		let source = app.world_mut().spawn(VirtualPadController::new(PadDevices::default())).id();
		app.world_mut()
			.resource_mut::<maybraid_controllers_core::InputRouter>()
			.assign_actor(InputSource::virtual_pad(source), ActorTarget::new(user));
		source
	}

	fn hold_rb(buttons: &[PadButton]) -> VirtualPad {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		for button in buttons {
			pad.hold_digital(*button);
		}
		pad.finish_digital();
		pad
	}

	fn send_pad(app: &mut App, source: Entity, sequence: u64, pad: VirtualPad) {
		app.world_mut().write_message(VirtualPadEvent {
			source,
			sequence,
			delta: std::time::Duration::from_millis(20),
			pad,
		});
	}

	fn overlay_of(app: &App, user: Entity) -> SkillMapOverlay {
		*app.world().get::<SkillMapOverlay>(user).expect("overlay")
	}

	fn hud_width(app: &App, hud: Entity) -> Val {
		app.world().get::<Node>(hud).expect("hud").width
	}

	fn drive_rb(app: &mut App, source: Entity, sequence: u64, hold: bool) {
		if hold {
			send_pad(app, source, sequence, hold_rb(&[PadButton::BumperFire]));
		} else {
			send_pad(app, source, sequence, hold_rb(&[]));
		}
		app.update();
	}

	#[test]
	fn laufer_rb_hold_expands_and_release_collapses() -> anyhow::Result<()> {
		let mut app = overlay_app();
		let (user, hud) = spawn_overlay_user(&mut app, true);
		let source = assign_pad(&mut app, user);
		drive_rb(&mut app, source, 1, false);
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Collapsed);
		assert_eq!(hud_width(&app, hud), Val::Px(VIEWPORT_PX));
		drive_rb(&mut app, source, 2, true);
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Expanded);
		assert_eq!(hud_width(&app, hud), Val::Percent(100.0));
		drive_rb(&mut app, source, 3, false);
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Collapsed);
		assert_eq!(hud_width(&app, hud), Val::Px(VIEWPORT_PX));
		Ok(())
	}

	#[test]
	fn tanzer_rb_hold_expands_and_release_collapses() -> anyhow::Result<()> {
		let mut app = overlay_app();
		let (user, hud) = spawn_overlay_user(&mut app, false);
		let source = assign_pad(&mut app, user);
		drive_rb(&mut app, source, 1, false);
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Collapsed);
		assert_eq!(hud_width(&app, hud), Val::Px(VIEWPORT_PX));
		assert!(
			!app.world().get::<SkillMapHeld>(user).expect("held").0,
			"Tanzer must not keep the map live without RB"
		);
		drive_rb(&mut app, source, 2, true);
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Expanded);
		assert_eq!(hud_width(&app, hud), Val::Percent(100.0));
		drive_rb(&mut app, source, 3, false);
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Collapsed);
		assert_eq!(hud_width(&app, hud), Val::Px(VIEWPORT_PX));
		Ok(())
	}

	#[test]
	fn bumper_chord_does_not_expand_the_overlay() -> anyhow::Result<()> {
		let mut app = overlay_app();
		let (user, _) = spawn_overlay_user(&mut app, true);
		let source = assign_pad(&mut app, user);
		send_pad(&mut app, source, 1, hold_rb(&[PadButton::BumperFocus, PadButton::BumperFire]));
		app.update();
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Collapsed);
		Ok(())
	}

	struct TunnelProbe {
		user: Entity,
		gameplay: Entity,
		firearm: Entity,
		chrome: Entity,
		camera: Entity,
	}

	fn spawn_tunnel_user(app: &mut App, ordinary: bool) -> TunnelProbe {
		let (user, _) = spawn_overlay_user(app, ordinary);
		let session = app.world().get::<SkillMapUser>(user).expect("user").maps;
		let member = SkillMapMember { user, session };
		let gameplay = app.world_mut().spawn((GameplayHud, Visibility::Inherited)).id();
		let firearm = app
			.world_mut()
			.spawn((SkillMapFirearmHud, GameplayHud, member, Visibility::Inherited))
			.id();
		let chrome = app
			.world_mut()
			.spawn((SkillMapCollapsedChrome, member, Visibility::Inherited))
			.id();
		let camera = app.world_mut().spawn(Camera3d::default()).id();
		TunnelProbe { user, gameplay, firearm, chrome, camera }
	}

	fn visibility_of(app: &App, entity: Entity) -> Visibility {
		*app.world().get::<Visibility>(entity).expect("visibility")
	}

	fn tunnel_of(app: &App) -> SkillMapTunnelVision {
		*app.world().resource::<SkillMapTunnelVision>()
	}

	fn blur_on(app: &App, camera: Entity) -> Option<SkillMapTunnelVisionMaterial> {
		app.world().get::<SkillMapTunnelVisionMaterial>(camera).copied()
	}

	fn assert_tunnel_idle(app: &App, probe: &TunnelProbe) {
		assert_eq!(overlay_of(app, probe.user), SkillMapOverlay::Collapsed);
		assert!(!tunnel_of(app).hides_hud());
		assert!(!tunnel_of(app).blur_enabled());
		assert_eq!(visibility_of(app, probe.gameplay), Visibility::Inherited);
		assert_eq!(visibility_of(app, probe.firearm), Visibility::Inherited);
		assert_eq!(visibility_of(app, probe.chrome), Visibility::Inherited);
		assert!(blur_on(app, probe.camera).is_none());
	}

	fn assert_tunnel_active(app: &App, probe: &TunnelProbe) {
		assert_eq!(overlay_of(app, probe.user), SkillMapOverlay::Expanded);
		assert!(tunnel_of(app).hides_hud());
		assert!(tunnel_of(app).blur_enabled());
		assert_eq!(visibility_of(app, probe.gameplay), Visibility::Hidden);
		assert_eq!(visibility_of(app, probe.firearm), Visibility::Hidden);
		assert_eq!(visibility_of(app, probe.chrome), Visibility::Hidden);
		let blur = blur_on(app, probe.camera).expect("edge blur");
		assert_eq!(blur.strength, crate::TUNNEL_VISION_STRENGTH);
		assert_eq!(blur.radius, crate::TUNNEL_VISION_RADIUS);
		assert_eq!(blur.darken, crate::TUNNEL_VISION_DARKEN);
		assert!(blur.amount > 0.0);
	}

	fn assert_rb_hold_hides_hud_and_enables_blur(
		app: &mut App,
		probe: &TunnelProbe,
		source: Entity,
	) {
		drive_rb(app, source, 1, false);
		assert_tunnel_idle(app, probe);
		drive_rb(app, source, 2, true);
		assert_tunnel_active(app, probe);
		drive_rb(app, source, 3, false);
		assert_tunnel_idle(app, probe);
	}

	#[test]
	fn laufer_rb_hold_hides_hud_and_enables_edge_blur() -> anyhow::Result<()> {
		let mut app = tunnel_app();
		let probe = spawn_tunnel_user(&mut app, true);
		let source = assign_pad(&mut app, probe.user);
		assert_rb_hold_hides_hud_and_enables_blur(&mut app, &probe, source);
		Ok(())
	}

	#[test]
	fn tanzer_rb_hold_hides_hud_and_enables_edge_blur() -> anyhow::Result<()> {
		let mut app = tunnel_app();
		let probe = spawn_tunnel_user(&mut app, false);
		let source = assign_pad(&mut app, probe.user);
		assert_rb_hold_hides_hud_and_enables_blur(&mut app, &probe, source);
		Ok(())
	}

	#[test]
	fn bumper_chord_does_not_hide_hud_or_enable_edge_blur() -> anyhow::Result<()> {
		let mut app = tunnel_app();
		let probe = spawn_tunnel_user(&mut app, true);
		let source = assign_pad(&mut app, probe.user);
		send_pad(&mut app, source, 1, hold_rb(&[PadButton::BumperFocus, PadButton::BumperFire]));
		app.update();
		assert_tunnel_idle(&app, &probe);
		Ok(())
	}

	fn assert_activate_once_per_hold(app: &mut App, user: Entity, source: Entity) {
		drive_rb(app, source, 1, false);
		assert_eq!(overlay_of(app, user), SkillMapOverlay::Collapsed);
		assert_eq!(activations(app), 0);
		drive_rb(app, source, 2, true);
		assert_eq!(overlay_of(app, user), SkillMapOverlay::Expanded);
		assert_eq!(activations(app), 1, "RB hold must play the activate clip once");
		drive_rb(app, source, 3, true);
		assert_eq!(overlay_of(app, user), SkillMapOverlay::Expanded);
		assert_eq!(activations(app), 1, "held RB must not retrigger the activate clip");
		drive_rb(app, source, 4, false);
		assert_eq!(overlay_of(app, user), SkillMapOverlay::Collapsed);
		assert_eq!(activations(app), 1);
		drive_rb(app, source, 5, true);
		assert_eq!(overlay_of(app, user), SkillMapOverlay::Expanded);
		assert_eq!(activations(app), 2, "a second hold is a new activation");
	}

	#[test]
	fn laufer_rb_hold_plays_activate_sound_once() -> anyhow::Result<()> {
		let mut app = overlay_sound_app();
		let (user, _) = spawn_overlay_user(&mut app, true);
		let source = assign_pad(&mut app, user);
		assert_activate_once_per_hold(&mut app, user, source);
		Ok(())
	}

	#[test]
	fn tanzer_rb_hold_plays_activate_sound_once() -> anyhow::Result<()> {
		let mut app = overlay_sound_app();
		let (user, _) = spawn_overlay_user(&mut app, false);
		let source = assign_pad(&mut app, user);
		assert_activate_once_per_hold(&mut app, user, source);
		Ok(())
	}

	#[test]
	fn bumper_chord_does_not_play_activate_sound() -> anyhow::Result<()> {
		let mut app = overlay_sound_app();
		let (user, _) = spawn_overlay_user(&mut app, true);
		let source = assign_pad(&mut app, user);
		send_pad(&mut app, source, 1, hold_rb(&[PadButton::BumperFocus, PadButton::BumperFire]));
		app.update();
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Collapsed);
		assert_eq!(activations(&app), 0, "LB+RB chord must not play the overlay clip");
		Ok(())
	}

	fn feed_of(app: &App, user: Entity) -> crate::feed::FeedState {
		let session = app.world().get::<SkillMapUser>(user).expect("user").maps;
		app.world().get::<crate::feed::FeedState>(session).expect("feed").clone()
	}

	fn assert_rb_hold_starts_the_feed(app: &mut App, user: Entity, source: Entity) {
		drive_rb(app, source, 1, false);
		assert!(!feed_of(app, user).started(), "idle maps must wait for RB");
		drive_rb(app, source, 2, true);
		assert!(feed_of(app, user).started(), "RB hold must start the feed without a flick");
		let session = app.world().get::<SkillMapUser>(user).expect("user").maps;
		app.world_mut()
			.get_mut::<crate::feed::FeedState>(session)
			.expect("feed")
			.advance(0.4);
		drive_rb(app, source, 3, true);
		let held = feed_of(app, user);
		assert!(held.started());
		assert!(held.elapsed() >= 0.4, "held RB must keep the same phrase running, not restart it");
		drive_rb(app, source, 4, false);
		let paused = feed_of(app, user);
		assert!(paused.started());
		assert!((paused.elapsed() - held.elapsed()).abs() < 1e-3, "release must pause the clock");
		drive_rb(app, source, 5, true);
		let restarted = feed_of(app, user);
		assert!(restarted.started());
		assert_eq!(restarted.hits(), 0);
		assert!(
			restarted.elapsed() < 0.05,
			"re-holding RB must restart the phrase, got {}",
			restarted.elapsed()
		);
	}

	#[test]
	fn laufer_rb_hold_starts_the_feed_immediately() -> anyhow::Result<()> {
		let mut app = overlay_app();
		let (user, _) = spawn_overlay_user(&mut app, true);
		let source = assign_pad(&mut app, user);
		assert_rb_hold_starts_the_feed(&mut app, user, source);
		Ok(())
	}

	#[test]
	fn tanzer_rb_hold_starts_the_feed_immediately() -> anyhow::Result<()> {
		let mut app = overlay_app();
		let (user, _) = spawn_overlay_user(&mut app, false);
		let source = assign_pad(&mut app, user);
		assert_rb_hold_starts_the_feed(&mut app, user, source);
		Ok(())
	}

	#[test]
	fn bumper_chord_does_not_start_the_feed() -> anyhow::Result<()> {
		let mut app = overlay_app();
		let (user, _) = spawn_overlay_user(&mut app, true);
		let source = assign_pad(&mut app, user);
		send_pad(&mut app, source, 1, hold_rb(&[PadButton::BumperFocus, PadButton::BumperFire]));
		app.update();
		assert_eq!(overlay_of(&app, user), SkillMapOverlay::Collapsed);
		assert!(!feed_of(&app, user).started(), "LB+RB chord must not start the feed");
		Ok(())
	}

	#[test]
	fn overlay_veil_is_a_light_hint() {
		assert!((OVERLAY_GLASS - 0.056).abs() < 1e-6);
		assert!((OVERLAY_TILE_AMOUNT - 1.0).abs() < 1e-6);
		assert!((OVERLAY_CLEAR_ALPHA - 0.020).abs() < 1e-6);
		assert!((OVERLAY_FRAME_FILL_ALPHA - 0.008).abs() < 1e-6);
		assert!((OVERLAY_WARP - 0.28).abs() < 1e-6);
	}

	#[test]
	fn lowering_tile_overlay_amount_increases_coverage() {
		let opaque = SkillMapTileMaterial::overlay_alpha(1.0, 0.0);
		let half = SkillMapTileMaterial::overlay_alpha(1.0, OVERLAY_TILE_AMOUNT_NINE_F227);
		let full = SkillMapTileMaterial::overlay_alpha(1.0, 1.0);
		let mistaken = SkillMapTileMaterial::overlay_alpha(1.0, 0.35);
		assert!((opaque - 1.0).abs() < 1e-5);
		assert!(
			full < half,
			"style.w blends toward overlay translucency; 1.0 must be clearer than 0.50"
		);
		assert!(
			half < mistaken,
			"dropping tile amount from 0.50 to 0.35 must not be treated as a lighter veil"
		);
		assert!(mistaken < opaque);
	}

	#[test]
	fn expanded_overlay_is_clearer_than_the_nine_f227_tile_veil() {
		let now = SkillMapOverlay::Expanded.pane_alpha(1.0);
		let nine_f227_sample =
			SkillMapTileMaterial::overlay_alpha(1.0, OVERLAY_TILE_AMOUNT_NINE_F227);
		let nine_f227 =
			OverlayParams { amount: 1.0, warp: OVERLAY_WARP, glass: OVERLAY_GLASS, _pad: 0.0 }
				.composited_alpha(nine_f227_sample, 0.0, 0.5);
		let nine_13ff =
			OverlayParams { amount: 1.0, warp: OVERLAY_WARP, glass: OVERLAY_GLASS, _pad: 0.0 }
				.composited_alpha(1.0 * 0.28 + 0.05, 0.0, 0.5);
		assert!(
			now < nine_f227,
			"expanded pane alpha {now} must stay below the 9f22704 veil {nine_f227}"
		);
		assert!(
			now < nine_13ff,
			"expanded pane alpha {now} must stay below the 913ff6fe4 veil {nine_13ff}"
		);
		assert!(now <= 0.21, "background pane must stay a light veil, got {now}");
		assert!(now < 0.29, "must drop below the 913ff6fe4 ~0.29 pane, got {now}");
		assert!((SkillMapOverlay::Collapsed.pane_alpha(1.0) - 1.0).abs() < 1e-5);
	}

	#[test]
	fn overlay_marks_stay_strong() {
		let expanded = SkillMapOverlay::Expanded;
		assert!(
			expanded.mark_alpha(1.0) >= OVERLAY_MARK_ALPHA_MIN,
			"feed arrows / power tiles must stay readable, got {}",
			expanded.mark_alpha(1.0)
		);
		assert!(
			expanded.mark_alpha(1.0) > expanded.pane_alpha(1.0) + 0.5,
			"marks must sit well above the land veil"
		);
		assert!(!SkillMapTileMaterial::overlay_veils_kind(crate::tile_material::TILE_KIND_FIRE));
		assert!(!SkillMapTileMaterial::overlay_veils_kind(crate::tile_material::TILE_KIND_WAVE));
		assert!(!SkillMapTileMaterial::overlay_veils_kind(crate::tile_material::TILE_KIND_WATER));
		assert!(!SkillMapTileMaterial::overlay_veils_kind(crate::tile_material::TILE_KIND_CURSOR));
		assert!(!SkillMapTileMaterial::overlay_veils_kind(crate::tile_material::TILE_KIND_ROCK));
		assert!(!SkillMapTileMaterial::overlay_veils_kind(crate::tile_material::TILE_KIND_COSMO));
		assert!(SkillMapTileMaterial::overlay_veils_kind(
			crate::tile_material::TILE_KIND_LAND_ROCK
		));
		assert_eq!(
			SkillMapTileMaterial::overlay_alpha_for_kind(
				1.0,
				1.0,
				crate::tile_material::TILE_KIND_FIRE
			),
			1.0
		);
		assert!(
			crate::viewport::DEBRAID_OVERLAY_ALPHA >= OVERLAY_MARK_ALPHA_MIN,
			"debraid HUD is off the map RT and must stay strong"
		);
		assert!((FeedArrow::hit_line_color().to_srgba().alpha - 1.0).abs() < 1e-5);
	}
}
