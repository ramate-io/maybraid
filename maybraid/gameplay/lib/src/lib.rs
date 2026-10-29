//! Gameplay assembly: session, equipment, item runtimes, and cross-domain
//! adapters. Adapters translate or coordinate; they hold no domain state.

pub mod actor_lifecycle;
pub mod equipment;
pub mod powers;
pub mod schedule;

use bevy::prelude::*;
use firearm_user::FirearmUserPlugin;
use fisticuffs_user::{FisticuffsUserPlugin, FisticuffsUserSystems};
use grenade_user::GrenadeUserPlugin;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_session::SessionPlugin;
use maybraid_skill_map::SkillMapPlugin;
use tanz_user::TanzUserSystems;

pub use actor_lifecycle::ActorDomainTeardown;
pub use equipment::ItemRuntimes;

#[derive(Resource, Default)]
struct GameplayRuntime;

/// Headless-safe session + item-runtime registration shared by
/// [`GameplayPlugin`] and stack-tip tests. Rendering / audio stay on the
/// full item plugins added by [`GameplayPlugin`].
///
/// Idempotent: a second call, or a later item-plugin `register_*_runtime`,
/// does not add systems again.
pub fn register_gameplay_runtime(app: &mut App) {
	if app.world().contains_resource::<GameplayRuntime>() {
		return;
	}
	app.init_resource::<GameplayRuntime>();
	if !app.is_plugin_added::<CharacterControllerPlugin>() {
		app.add_plugins(CharacterControllerPlugin);
	}
	if !app.is_plugin_added::<SessionPlugin>() {
		app.add_plugins(SessionPlugin);
	}
	if !app.is_plugin_added::<lubikans::LubikansPlugin>() {
		app.add_plugins(lubikans::LubikansPlugin);
	}
	if !app.is_plugin_added::<lubikans_damage::LubikanDamagePlugin>() {
		app.add_plugins(lubikans_damage::LubikanDamagePlugin);
	}
	schedule::configure(app);
	equipment::configure(app);
	powers::configure(app);
	actor_lifecycle::configure(app);
	maybraid_spawns_player::register_player_spawn_runtime(app);
	maybraid_spawns_placement::register_placement_runtime(app);
	maybraid_spawns_placement::register_placement_physics(app);
	maybraid_spawns_actors::register_actor_spawn_runtime(app);
	maybraid_spawns_projectiles::register_projectile_spawn_runtime(app);
	maybraid_spawns_grenades::register_grenade_spawn_runtime(app);
}

/// Give the full-body jab clip precedence over Tanz animation while both
/// feature plugins are installed.
pub fn configure_user_animation_order(app: &mut App) {
	app.configure_sets(Update, FisticuffsUserSystems::Animate.after(TanzUserSystems::Animate));
}

/// Assembles session, equipment, and item-domain runtimes. Placement owns
/// support / clearance. World keeps first-wave, map, and activation adapters.
///
/// Item *runtimes* are registered here (and only here in the assembled game).
/// Item *presentation* plugins are added after the runtime so either install
/// order is safe.
pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
	fn build(&self, app: &mut App) {
		register_gameplay_runtime(app);
		if !app.is_plugin_added::<SkillMapPlugin>() {
			app.add_plugins(SkillMapPlugin);
		}
		if !app.is_plugin_added::<FirearmUserPlugin>() {
			app.add_plugins(FirearmUserPlugin);
		}
		if !app.is_plugin_added::<GrenadeUserPlugin>() {
			app.add_plugins(GrenadeUserPlugin);
		}
		if !app.is_plugin_added::<FisticuffsUserPlugin>() {
			app.add_plugins(FisticuffsUserPlugin);
		}
		configure_user_animation_order(app);
	}
}

#[cfg(test)]
mod controller_stack;

#[cfg(test)]
mod tests {
	use super::*;
	use characters::{
		AnimClip, AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, CharacterRig, CharacterRigRole,
		CharacterRoot, JabParams, MemberOf,
	};
	use firearm_user::register_firearm_user_runtime;
	use fisticuffs_user::{AttackSpec, AttackState, FisticuffsUser, Side, JAB_STARTUP_END};
	use grenade_user::register_grenade_user_runtime;
	use player::Player;
	use tanz_user::{
		ActiveTanz, ControlMode, TanzDefinitionExt, TanzExecution, TanzId, TanzPhase, TanzUser,
		TanzUserSystems,
	};

	fn headless() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app
	}

	#[test]
	fn register_gameplay_runtime_is_idempotent() {
		let mut app = headless();
		register_gameplay_runtime(&mut app);
		register_gameplay_runtime(&mut app);
	}

	#[test]
	fn item_runtime_after_gameplay_is_idempotent() {
		let mut app = headless();
		register_gameplay_runtime(&mut app);
		register_firearm_user_runtime(&mut app);
		register_grenade_user_runtime(&mut app);
	}

	#[test]
	fn gameplay_after_item_runtime_is_idempotent() {
		let mut app = headless();
		register_firearm_user_runtime(&mut app);
		register_grenade_user_runtime(&mut app);
		register_gameplay_runtime(&mut app);
	}

	#[test]
	fn jab_animation_wins_over_tanz_and_tanz_resumes_afterward() -> anyhow::Result<()> {
		let mut app = headless();
		configure_user_animation_order(&mut app);
		app.add_systems(Update, tanz_user::drive_tanz_animation.in_set(TanzUserSystems::Animate))
			.add_systems(
				Update,
				fisticuffs_user::apply_fisticuffs_visual
					.in_set(fisticuffs_user::FisticuffsUserSystems::Animate),
			);

		let mut attack = AttackState::default();
		attack.request(AttackSpec::jab(Side::Left));
		attack.tick(JAB_STARTUP_END * 0.5);
		let progress = attack
			.clip_progress()
			.ok_or_else(|| anyhow::anyhow!("active jab should have clip progress"))?;
		let target = attack
			.aim()
			.ok_or_else(|| anyhow::anyhow!("active jab should have latched aim"))?
			.target;
		let actor = app
			.world_mut()
			.spawn((
				Player,
				FisticuffsUser::default(),
				TanzUser { mode: ControlMode::Tanzer, ..default() },
				TanzExecution {
					active: Some(ActiveTanz {
						id: TanzId::LeftStrafe,
						age: 0.12,
						phase: TanzPhase::Active,
						applied: true,
					}),
					..default()
				},
			))
			.id();
		app.world_mut()
			.get_mut::<FisticuffsUser>(actor)
			.ok_or_else(|| anyhow::anyhow!("player has no fisticuffs user"))?
			.state = attack;
		let root = app.world_mut().spawn((CharacterRoot, ChildOf(actor))).id();
		let body = app
			.world_mut()
			.spawn((
				CharacterRig { role: CharacterRigRole::Body, ..default() },
				MemberOf(root),
				AnimMailbox::new(Transform::IDENTITY),
				AnimRefRoot(AnimRef::still()),
				AnimProgress(0.0),
			))
			.id();

		app.update();
		let expected_jab = AnimRef::new(AnimClip::Jab(JabParams {
			side: Side::Left,
			backswing: character_animations::animations::DEFAULT_BACKSWING,
			target,
		}));
		let body_anim = app
			.world()
			.get::<AnimRefRoot>(body)
			.ok_or_else(|| anyhow::anyhow!("body has no animation reference"))?;
		assert_eq!(body_anim.0, expected_jab);
		let body_progress = app
			.world()
			.get::<AnimProgress>(body)
			.ok_or_else(|| anyhow::anyhow!("body has no animation progress"))?;
		assert_eq!(body_progress.0, progress);

		app.world_mut()
			.get_mut::<FisticuffsUser>(actor)
			.ok_or_else(|| anyhow::anyhow!("player lost fisticuffs user"))?
			.state
			.cancel();
		app.update();
		let tanz = TanzId::LeftStrafe.definition();
		let expected_tanz = AnimRef::new(tanz.clip);
		let expected_progress = 0.12 / tanz.timing.total();
		let body_anim = app
			.world()
			.get::<AnimRefRoot>(body)
			.ok_or_else(|| anyhow::anyhow!("body has no resumed Tanz animation"))?;
		assert_eq!(body_anim.0, expected_tanz);
		let body_progress = app
			.world()
			.get::<AnimProgress>(body)
			.ok_or_else(|| anyhow::anyhow!("body has no resumed Tanz progress"))?;
		assert!((body_progress.0 - expected_progress).abs() < 1e-6);
		Ok(())
	}
}
