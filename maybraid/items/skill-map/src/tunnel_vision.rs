//! Tunnel vision while the stained-glass overlay is expanded: hide the rest of
//! the HUD and fade a vignette-style edge blur on the 3D camera.

use bevy::asset::embedded_asset;
use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::render::RenderApp;
use bevy::shader::ShaderRef;
use menu_components::GameplayHud;

use crate::overlay::SkillMapOverlay;
use crate::user::SkillMapUser;
use crate::viewport::SkillMapCollapsedChrome;

const TUNNEL_SHADER: &str = concat!("embedded://", env!("CARGO_CRATE_NAME"), "/tunnel_vision.wesl");

/// UV blur radius at the rim when [`Self::amount`] is 1.
pub const TUNNEL_VISION_STRENGTH: f32 = 0.020;
/// Sharp disk as a fraction of centre→mid-edge. Beyond this, blur + darken ramp.
pub const TUNNEL_VISION_RADIUS: f32 = 0.40;
/// How much the rim darkens (1 = black at the edge).
pub const TUNNEL_VISION_DARKEN: f32 = 0.32;
/// Seconds to fade the blur in or out with the overlay.
pub const TUNNEL_VISION_FADE_SECS: f32 = 0.16;

/// Smoothed tunnel-vision amount driven by the skill-map overlay.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct SkillMapTunnelVision {
	amount: f32,
	target: f32,
}

impl SkillMapTunnelVision {
	pub fn amount(self) -> f32 {
		self.amount
	}

	pub fn target(self) -> f32 {
		self.target
	}

	pub fn hides_hud(self) -> bool {
		self.target > 0.5
	}

	pub fn blur_enabled(self) -> bool {
		self.target > 0.0 || self.amount > 0.001
	}

	pub fn set_target(&mut self, expanded: bool) {
		self.target = if expanded { 1.0 } else { 0.0 };
	}

	pub fn tick(&mut self, dt: f32) {
		if dt <= f32::EPSILON {
			self.amount = self.target;
			return;
		}
		self.amount = Self::move_toward(self.amount, self.target, dt / TUNNEL_VISION_FADE_SECS);
	}

	fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
		let delta = target - current;
		if delta.abs() <= max_delta {
			target
		} else {
			current + delta.signum() * max_delta
		}
	}

	/// CPU mirror of `tunnel_vision.wesl` `edge_weight` (circular, `smoothstep`).
	pub fn edge_weight(self, uv: Vec2) -> f32 {
		SkillMapTunnelVisionMaterial::from_vision(self).edge_weight(uv)
	}
}

/// Fullscreen post-process on [`Camera3d`]. Samples the main view and blurs /
/// darkens toward the rim.
#[derive(Component, ExtractComponent, Clone, Copy, Debug, ShaderType, Default, PartialEq)]
#[extract_app(RenderApp)]
pub struct SkillMapTunnelVisionMaterial {
	pub amount: f32,
	pub strength: f32,
	pub radius: f32,
	pub darken: f32,
}

impl SkillMapTunnelVisionMaterial {
	pub fn from_vision(vision: SkillMapTunnelVision) -> Self {
		Self {
			amount: vision.amount,
			strength: TUNNEL_VISION_STRENGTH,
			radius: TUNNEL_VISION_RADIUS,
			darken: TUNNEL_VISION_DARKEN,
		}
	}

	pub fn edge_weight(self, uv: Vec2) -> f32 {
		let d = (uv - Vec2::splat(0.5)).length() * 2.0;
		let span = (1.0 - self.radius).max(1e-5);
		let t = ((d - self.radius) / span).clamp(0.0, 1.0);
		let smooth = t * t * (3.0 - 2.0 * t);
		smooth * self.amount.clamp(0.0, 1.0)
	}
}

impl FullscreenMaterial for SkillMapTunnelVisionMaterial {
	fn fragment_shader() -> ShaderRef {
		TUNNEL_SHADER.into()
	}
}

pub struct SkillMapTunnelVisionPlugin;

impl Plugin for SkillMapTunnelVisionPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "tunnel_vision.wesl");
		app.add_plugins(FullscreenMaterialPlugin::<SkillMapTunnelVisionMaterial>::default())
			.init_resource::<SkillMapTunnelVision>()
			.add_systems(
				PostUpdate,
				(tick_tunnel_vision, hide_gameplay_hud, apply_tunnel_vision_material).chain(),
			);
	}
}

/// Snap the fade target to the overlay the same frame RB expands or collapses it.
pub(crate) fn sync_tunnel_vision_target(
	mut vision: ResMut<SkillMapTunnelVision>,
	overlays: Query<&SkillMapOverlay, With<SkillMapUser>>,
	mut chrome: Query<&mut Visibility, With<SkillMapCollapsedChrome>>,
) {
	let expanded = overlays.iter().any(|overlay| overlay.expanded());
	vision.set_target(expanded);
	let chrome_vis = if expanded { Visibility::Hidden } else { Visibility::Inherited };
	for mut visibility in &mut chrome {
		if *visibility != chrome_vis {
			*visibility = chrome_vis;
		}
	}
}

pub(crate) fn tick_tunnel_vision(time: Res<Time>, mut vision: ResMut<SkillMapTunnelVision>) {
	vision.tick(time.delta_secs());
}

#[derive(Component, Clone, Copy)]
pub(crate) struct GameplayHudRest(Visibility);

/// Hide stamped gameplay HUD after each HUD's own Update sync has run.
/// Restores the pre-hide visibility on release so cook / Tanz / combat sync
/// is not overwritten while the overlay is down.
pub(crate) fn hide_gameplay_hud(
	mut commands: Commands,
	vision: Res<SkillMapTunnelVision>,
	mut huds: Query<(Entity, &mut Visibility, Option<&GameplayHudRest>), With<GameplayHud>>,
) {
	let hide = vision.hides_hud();
	for (entity, mut visibility, rest) in &mut huds {
		if hide {
			if rest.is_none() && *visibility != Visibility::Hidden {
				commands.entity(entity).insert(GameplayHudRest(*visibility));
			}
			if *visibility != Visibility::Hidden {
				*visibility = Visibility::Hidden;
			}
		} else if let Some(rest) = rest {
			*visibility = rest.0;
			commands.entity(entity).remove::<GameplayHudRest>();
		}
	}
}

pub(crate) fn apply_tunnel_vision_material(
	mut commands: Commands,
	vision: Res<SkillMapTunnelVision>,
	cameras: Query<Entity, With<Camera3d>>,
) {
	if vision.blur_enabled() {
		let material = SkillMapTunnelVisionMaterial::from_vision(*vision);
		for entity in &cameras {
			commands.entity(entity).insert(material);
		}
	} else {
		for entity in &cameras {
			commands.entity(entity).remove::<SkillMapTunnelVisionMaterial>();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn centre_stays_sharp_and_the_rim_blurs() {
		let mut vision = SkillMapTunnelVision::default();
		vision.set_target(true);
		vision.tick(1.0);
		assert!((vision.amount() - 1.0).abs() < 1e-5);
		assert!(vision.edge_weight(Vec2::splat(0.5)) < 1e-5);
		assert!(vision.edge_weight(Vec2::new(0.0, 0.5)) > 0.95);
		assert!(vision.edge_weight(Vec2::ZERO) > 0.95);
		assert!(TUNNEL_VISION_STRENGTH > 0.0);
		assert!(TUNNEL_VISION_RADIUS > 0.2);
		assert!(TUNNEL_VISION_RADIUS < 0.7);
		assert!(TUNNEL_VISION_DARKEN > 0.0);
		assert!(TUNNEL_VISION_DARKEN < 0.6);
	}

	#[test]
	fn shader_ref_matches_embedded_asset_path() {
		let path = bevy::asset::embedded_path!("tunnel_vision.wesl");
		let registered = path.to_str().unwrap().replace('\\', "/");
		assert_eq!(
			registered,
			format!("{}/tunnel_vision.wesl", env!("CARGO_CRATE_NAME")),
			"FullscreenMaterial ShaderRef must use this path; Bevy embeds subdirectory WGSL under src/"
		);
		assert!(TUNNEL_SHADER.ends_with(&registered));
	}

	#[test]
	fn fade_reaches_the_target_and_snaps_when_dt_is_zero() {
		let mut vision = SkillMapTunnelVision::default();
		vision.set_target(true);
		vision.tick(TUNNEL_VISION_FADE_SECS * 0.25);
		assert!(vision.amount() > 0.2);
		assert!(vision.amount() < 0.4);
		vision.tick(0.0);
		assert!((vision.amount() - 1.0).abs() < 1e-5);
		vision.set_target(false);
		vision.tick(0.0);
		assert_eq!(vision.amount(), 0.0);
		assert!(!vision.hides_hud());
		assert!(!vision.blur_enabled());
	}
}
