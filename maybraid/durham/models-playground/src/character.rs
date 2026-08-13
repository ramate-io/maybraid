//! Crozon character visual on the terrain player (replaces the capsule mesh).

use bevy::ecs::query::Has;
use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use clap::ValueEnum;
use crozon_characters::{
	character_bounds, spawn_character_components,
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
	AnimId, AnimRef, AnimRefRoot, CharacterMembers, CharacterRecipe, CharacterRig,
	CharacterRigRole, CharacterRoot, RigSkeletonKind,
};
use game_commands::ui::GameCommandStatusText;

use crate::commands::RequestModeCharacter;
use crate::player::{capsule_half_height, Grounded, Player, PlayerCapsule};
use avian3d::prelude::LinearVelocity;

const WALK_SPEED: f32 = 1.0;
const RUN_SPEED: f32 = 5.0;
const FACE_SPEED: f32 = 0.35;

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

pub(crate) fn apply_set_character(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	requests: Query<(Entity, &RequestSetCharacter)>,
	players: Query<Entity, With<Player>>,
	visuals: Query<Entity, With<PlayerVisual>>,
	mut capsules: Query<&mut Visibility, With<PlayerCapsule>>,
) {
	let Ok(player) = players.single() else {
		for (entity, _) in &requests {
			commands.entity(entity).despawn();
		}
		return;
	};

	for (entity, request) in &requests {
		for visual in &visuals {
			commands.entity(visual).try_despawn();
		}
		for mut visibility in &mut capsules {
			*visibility = Visibility::Hidden;
		}

		let offset = Transform::from_translation(Vec3::new(0.0, -capsule_half_height(), 0.0));
		for spawned in spawn_species(&mut commands, request.species, offset) {
			commands.entity(spawned).insert((ChildOf(player), PlayerVisual));
		}
		commands.spawn(RequestModeCharacter);
		status.0 = format!(
			"set-character {} — mode character, WASD move, Space jump",
			request.species.label()
		);
		commands.entity(entity).despawn();
	}
}

/// Walk / run / jump on the body mailbox from player speed and grounded state.
pub(crate) fn drive_player_locomotion(
	mut commands: Commands,
	players: Query<(&LinearVelocity, Has<Grounded>), With<Player>>,
	mut visuals: Query<
		(&CharacterMembers, &mut Transform),
		(With<PlayerVisual>, With<CharacterRoot>),
	>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	let Ok((velocity, grounded)) = players.single() else {
		return;
	};
	let Ok((members, mut visual)) = visuals.single_mut() else {
		return;
	};

	let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
	let speed = horizontal.length();
	if speed > FACE_SPEED {
		visual.look_to(horizontal, Vec3::Y);
	}

	for member in members.iter() {
		let Ok(rig) = rigs.get(member) else {
			continue;
		};
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		let clip = locomotion_clip(rig.skeleton, grounded, speed);
		let desired = AnimRef::new(clip);
		let needs = match anims.get(member) {
			Ok(root) => root.0 != desired,
			Err(_) => true,
		};
		if needs {
			commands.entity(member).insert(AnimRefRoot(desired));
		}
	}
}

fn locomotion_clip(skeleton: RigSkeletonKind, grounded: bool, speed: f32) -> AnimId {
	match skeleton {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => {
			if !grounded {
				AnimId::Jump
			} else if speed > RUN_SPEED {
				AnimId::Run
			} else if speed > WALK_SPEED {
				AnimId::Walk
			} else {
				AnimId::Still
			}
		}
		RigSkeletonKind::Quadruped => {
			if speed > RUN_SPEED {
				AnimId::Gallop
			} else if speed > WALK_SPEED {
				AnimId::Run
			} else {
				AnimId::Still
			}
		}
		RigSkeletonKind::Forelimbed => {
			if speed > RUN_SPEED {
				AnimId::DorsoventralUndulation
			} else if speed > WALK_SPEED {
				AnimId::LateralUndulation
			} else {
				AnimId::Still
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
			spawn_character_components(commands, &clothed, transform, bounds)
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
