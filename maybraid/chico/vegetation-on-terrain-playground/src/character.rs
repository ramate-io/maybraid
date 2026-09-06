//! Crozon character visual on the terrain player (replaces the capsule mesh).

use bevy::ecs::query::Has;
use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use clap::ValueEnum;
use crozon_characters::{
	character_bounds,
	species::{
		braidman::BraidmanConfig, brenal::BrenalConfig, brodler::BrodlerConfig,
		brokker::BrokkerConfig, caole::CaoleConfig, chupri::ChupriConfig, claber::ClaberConfig,
		croconot::CroconotConfig, dui::DuiConfig, epiphant::EpiphantConfig, grener::GrenerConfig,
		hars::HarsConfig, kaller::KallerConfig, kappler::KapplerConfig, kispar::KisparConfig,
		lero::LeroConfig, lidder::LidderConfig, mistler::MistlerConfig, mygr::MygrConfig,
		sonyak::SonyakConfig, spibmom::SpibmomConfig, tapp::TappConfig, thumplus::ThumplusConfig,
		tipple::TippleConfig, topple::ToppleConfig, tuberwaber::TuberwaberConfig,
		wumbus::WumbusConfig, ylter::YilterConfig,
	},
	AnimClip, AnimId, AnimRef, AnimRefRoot, CharacterAppearance, CharacterHeading,
	CharacterMembers, CharacterRecipe, CharacterRig, CharacterRigRole, CharacterRoot,
	ComponentsOnly, RigSkeletonKind,
};
use game_commands::ui::GameCommandStatusText;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::commands::RequestModeCharacter;
use crate::player::{Grounded, Jumping, MoveWish, Player, PlayerCapsule, WalkableGround};
use avian3d::prelude::LinearVelocity;

const WALK_SPEED: f32 = 1.0;
const RUN_SPEED: f32 = 5.0;
const RUN_EXIT_SPEED: f32 = 4.5;

/// Species for `/set-character`. Default preview recipe, no concepts sliders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum CharacterSpecies {
	Braidman,
	Brenal,
	Caole,
	Epiphant,
	Hars,
	Yilter,
	Sonyak,
	Claber,
	Croconot,
	Brodler,
	Mygr,
	Dui,
	Lidder,
	Chupri,
	Brokker,
	Tipple,
	Topple,
	Kispar,
	Tapp,
	Kaller,
	Kappler,
	Wumbus,
	Lero,
	Spibmom,
	Grener,
	Thumplus,
	Mistler,
	Tuberwaber,
}

impl CharacterSpecies {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Braidman => "braidman",
			Self::Brenal => "brenal",
			Self::Caole => "caole",
			Self::Epiphant => "epiphant",
			Self::Hars => "hars",
			Self::Yilter => "ylter",
			Self::Sonyak => "sonyak",
			Self::Claber => "claber",
			Self::Croconot => "croconot",
			Self::Brodler => "brodler",
			Self::Mygr => "mygr",
			Self::Dui => "dui",
			Self::Lidder => "lidder",
			Self::Chupri => "chupri",
			Self::Brokker => "brokker",
			Self::Tipple => "tipple",
			Self::Topple => "topple",
			Self::Kispar => "kispar",
			Self::Tapp => "tapp",
			Self::Kaller => "kaller",
			Self::Kappler => "kappler",
			Self::Wumbus => "wumbus",
			Self::Lero => "lero",
			Self::Spibmom => "spibmom",
			Self::Grener => "grener",
			Self::Thumplus => "thumplus",
			Self::Mistler => "mistler",
			Self::Tuberwaber => "tuberwaber",
		}
	}
}

/// Nested character host parented to [`Player`].
#[derive(Component)]
pub struct PlayerVisual;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestSetCharacter {
	pub species: CharacterSpecies,
}

/// Replace the terrain player visual with a configured, inventory-clothed appearance.
#[derive(Component, Debug, Clone)]
pub struct RequestSetCharacterAppearance {
	pub appearance: CharacterAppearance,
}

pub(crate) fn apply_set_character(
	mut commands: Commands,
	mut status: Option<ResMut<GameCommandStatusText>>,
	species_requests: Query<(Entity, &RequestSetCharacter)>,
	appearance_requests: Query<(Entity, &RequestSetCharacterAppearance)>,
	players: Query<Entity, With<Player>>,
	visuals: Query<Entity, With<PlayerVisual>>,
	mut capsules: Query<&mut Visibility, With<PlayerCapsule>>,
) {
	let Ok(player) = players.single() else {
		for (entity, _) in &species_requests {
			commands.entity(entity).despawn();
		}
		for (entity, _) in &appearance_requests {
			commands.entity(entity).despawn();
		}
		return;
	};

	for (entity, request) in &species_requests {
		for visual in &visuals {
			commands.entity(visual).try_despawn();
		}
		for mut visibility in &mut capsules {
			*visibility = Visibility::Hidden;
		}

		for spawned in spawn_species(&mut commands, request.species, Transform::IDENTITY) {
			commands.entity(spawned).insert((ChildOf(player), PlayerVisual));
		}
		commands.spawn(RequestModeCharacter);
		crate::ui::write_status(
			&mut status,
			format!(
				"set-character {} — mode character, WASD move, Space jump",
				request.species.label()
			),
		);
		commands.entity(entity).despawn();
	}
	for (entity, request) in &appearance_requests {
		for visual in &visuals {
			commands.entity(visual).try_despawn();
		}
		for mut visibility in &mut capsules {
			*visibility = Visibility::Hidden;
		}

		for spawned in spawn_appearance(&mut commands, &request.appearance, Transform::IDENTITY) {
			commands.entity(spawned).insert((ChildOf(player), PlayerVisual));
		}
		commands.spawn(RequestModeCharacter);
		crate::ui::write_status(
			&mut status,
			format!(
				"set-character {} — mode character, WASD move, Space jump",
				request.appearance.species_id()
			),
		);
		commands.entity(entity).despawn();
	}
}

/// Walk / run / jump on the body mailbox from player speed and grounded state.
pub(crate) fn drive_player_locomotion(
	mut commands: Commands,
	time: Res<Time>,
	players: Query<
		(&LinearVelocity, &MoveWish, Has<Jumping>, Has<Grounded>, Option<&WalkableGround>),
		With<Player>,
	>,
	mut visuals: Query<
		(&CharacterMembers, &mut Transform, &mut CharacterHeading),
		(With<PlayerVisual>, With<CharacterRoot>),
	>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	let Ok((velocity, wish, jumping, grounded, walkable)) = players.single() else {
		return;
	};
	let Ok((members, mut visual, mut heading)) = visuals.single_mut() else {
		return;
	};

	let ground_normal = grounded.then(|| walkable.map(|ground| ground.normal)).flatten();
	let speed = locomotion_speed(velocity, ground_normal);
	heading.turn_toward(&mut visual, wish.0, time.delta_secs());

	for member in members.iter() {
		let Ok(rig) = rigs.get(member) else {
			continue;
		};
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		let current = anims.get(member).ok();
		let clip =
			locomotion_clip(rig.skeleton, jumping, speed, current.map(|root| root.0.clip.id()));
		let desired = AnimRef::new(clip);
		let needs = current.is_none_or(|root| root.0 != desired);
		if needs {
			commands.entity(member).insert(AnimRefRoot(desired));
		}
	}
}

fn locomotion_speed(velocity: &LinearVelocity, ground_normal: Option<Vec3>) -> f32 {
	if let Some(normal) = ground_normal.map(Vec3::normalize_or_zero) {
		if normal.length_squared() > 1e-8 {
			return (**velocity - normal * velocity.dot(normal)).length();
		}
	}
	Vec3::new(velocity.x, 0.0, velocity.z).length()
}

fn locomotion_clip(
	skeleton: RigSkeletonKind,
	jumping: bool,
	speed: f32,
	current: Option<AnimId>,
) -> AnimClip {
	let was_running = current.is_some_and(|id| {
		matches!(id, AnimId::Run | AnimId::Gallop | AnimId::DorsoventralUndulation)
	});
	let running = speed > if was_running { RUN_EXIT_SPEED } else { RUN_SPEED };
	match skeleton {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => {
			if jumping && speed > RUN_SPEED {
				AnimClip::leap()
			} else if jumping {
				AnimClip::jump()
			} else if running {
				AnimClip::run()
			} else if speed > WALK_SPEED {
				AnimClip::walk()
			} else {
				AnimClip::still()
			}
		}
		RigSkeletonKind::Quadruped => {
			if jumping {
				AnimClip::leap()
			} else if running {
				AnimClip::gallop()
			} else if speed > WALK_SPEED {
				AnimClip::quadruped_run()
			} else {
				AnimClip::still()
			}
		}
		RigSkeletonKind::Forelimbed => {
			if running {
				AnimClip::dorsoventral_undulation()
			} else if speed > WALK_SPEED {
				AnimClip::lateral_undulation()
			} else {
				AnimClip::still()
			}
		}
	}
}

fn spawn_species(
	commands: &mut Commands,
	species: CharacterSpecies,
	transform: Transform,
) -> Vec<Entity> {
	macro_rules! spawn_preview {
		($config:ty) => {{
			let clothed = CharacterRecipe::clothed(&<$config>::default_preview());
			let bounds = character_bounds(&clothed);
			let identity = Transform::IDENTITY;
			let lod_ref = LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &identity,
				current_transform: &identity,
				bounds: &bounds,
			};
			let host = ComponentsOnly(clothed);
			vec![commands
				.spawn_scene((
					host.host(&lod_ref),
					bsn! {
						template_value(transform)
					},
				))
				.id()]
		}};
	}
	match species {
		CharacterSpecies::Braidman => spawn_preview!(BraidmanConfig),
		CharacterSpecies::Brenal => spawn_preview!(BrenalConfig),
		CharacterSpecies::Caole => spawn_preview!(CaoleConfig),
		CharacterSpecies::Epiphant => spawn_preview!(EpiphantConfig),
		CharacterSpecies::Hars => spawn_preview!(HarsConfig),
		CharacterSpecies::Yilter => spawn_preview!(YilterConfig),
		CharacterSpecies::Sonyak => spawn_preview!(SonyakConfig),
		CharacterSpecies::Claber => spawn_preview!(ClaberConfig),
		CharacterSpecies::Croconot => spawn_preview!(CroconotConfig),
		CharacterSpecies::Brodler => spawn_preview!(BrodlerConfig),
		CharacterSpecies::Mygr => spawn_preview!(MygrConfig),
		CharacterSpecies::Dui => spawn_preview!(DuiConfig),
		CharacterSpecies::Lidder => spawn_preview!(LidderConfig),
		CharacterSpecies::Chupri => spawn_preview!(ChupriConfig),
		CharacterSpecies::Brokker => spawn_preview!(BrokkerConfig),
		CharacterSpecies::Tipple => spawn_preview!(TippleConfig),
		CharacterSpecies::Topple => spawn_preview!(ToppleConfig),
		CharacterSpecies::Kispar => spawn_preview!(KisparConfig),
		CharacterSpecies::Tapp => spawn_preview!(TappConfig),
		CharacterSpecies::Kaller => spawn_preview!(KallerConfig),
		CharacterSpecies::Kappler => spawn_preview!(KapplerConfig),
		CharacterSpecies::Wumbus => spawn_preview!(WumbusConfig),
		CharacterSpecies::Lero => spawn_preview!(LeroConfig),
		CharacterSpecies::Spibmom => spawn_preview!(SpibmomConfig),
		CharacterSpecies::Grener => spawn_preview!(GrenerConfig),
		CharacterSpecies::Thumplus => spawn_preview!(ThumplusConfig),
		CharacterSpecies::Mistler => spawn_preview!(MistlerConfig),
		CharacterSpecies::Tuberwaber => spawn_preview!(TuberwaberConfig),
	}
}

fn spawn_appearance(
	commands: &mut Commands,
	appearance: &CharacterAppearance,
	transform: Transform,
) -> Vec<Entity> {
	macro_rules! spawn_config {
		($config:expr) => {{
			let clothed = CharacterRecipe::clothed($config);
			let bounds = character_bounds(&clothed);
			let identity = Transform::IDENTITY;
			let lod_ref = LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &identity,
				current_transform: &identity,
				bounds: &bounds,
			};
			let host = ComponentsOnly(clothed);
			vec![commands
				.spawn_scene((
					host.host(&lod_ref),
					bsn! {
						template_value(transform)
					},
				))
				.id()]
		}};
	}
	match appearance {
		CharacterAppearance::Braidman(config) => spawn_config!(config),
		CharacterAppearance::Brodler(config) => spawn_config!(config),
		CharacterAppearance::Mygr(config) => spawn_config!(config),
		CharacterAppearance::Dui(config) => spawn_config!(config),
		CharacterAppearance::Wumbus(config) => spawn_config!(config),
		CharacterAppearance::Lero(config) => spawn_config!(config),
		CharacterAppearance::Spibmom(config) => spawn_config!(config),
		CharacterAppearance::Tuberwaber(config) => spawn_config!(config),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn grounded_locomotion_uses_surface_speed() {
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(slope.sin(), slope.cos(), 0.0);
		let downhill = LinearVelocity(Vec3::new(7.0 * slope.cos(), -7.0 * slope.sin(), 0.0));
		assert!((locomotion_speed(&downhill, Some(normal)) - 7.0).abs() < 1e-5);
		assert!(locomotion_speed(&downhill, None) < RUN_SPEED);
	}

	#[test]
	fn run_clip_has_exit_hysteresis() {
		let run = locomotion_clip(
			RigSkeletonKind::Humanoid,
			false,
			RUN_EXIT_SPEED + 0.1,
			Some(AnimId::Run),
		);
		let walk = locomotion_clip(
			RigSkeletonKind::Humanoid,
			false,
			RUN_EXIT_SPEED + 0.1,
			Some(AnimId::Walk),
		);
		assert_eq!(run.id(), AnimId::Run);
		assert_eq!(walk.id(), AnimId::Walk);
	}
}
