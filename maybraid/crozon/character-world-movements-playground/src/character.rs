//! Crozon character visual on the terrain player (replaces the capsule mesh).

use bevy::ecs::query::{Has, QueryFilter};
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
	AnimClip, AnimRef, AnimRefRoot, CharacterMembers, CharacterRecipe, CharacterRig,
	CharacterRigRole, CharacterRoot, ComponentsOnly, RigSkeletonKind,
};
use game_commands::ui::GameCommandStatusText;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::commands::RequestModeCharacter;
use crate::player::{
	controller_spawn_point, park_player_for_stampede, restore_player_controller,
	spawn_character_controller, CameraFollow, Jumping, MoveWish, Player, PlayerCapsule,
};
use crate::WorldBaseTerrain;
use avian3d::prelude::LinearVelocity;
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};

const WALK_SPEED: f32 = 1.0;
const RUN_SPEED: f32 = 5.0;
const FACE_DEADZONE: f32 = 0.05;
const TURN_RATE: f32 = 5.5;
/// World XZ spacing for [`RequestStampede`] so long quadrupeds do not overlap.
const STAMPEDE_SPACING: f32 = 4.0;

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
	/// Biped then quadruped preview recipes. Forelimbed species are omitted.
	pub const STAMPEDE: &'static [Self] = &[
		Self::Braidman,
		Self::Brodler,
		Self::Brokker,
		Self::Chupri,
		Self::Dui,
		Self::Kaller,
		Self::Kappler,
		Self::Kispar,
		Self::Lero,
		Self::Lidder,
		Self::Mygr,
		Self::Spibmom,
		Self::Tapp,
		Self::Tipple,
		Self::Topple,
		Self::Tuberwaber,
		Self::Wumbus,
		Self::Brenal,
		Self::Caole,
		Self::Claber,
		Self::Croconot,
		Self::Epiphant,
		Self::Hars,
		Self::Sonyak,
		Self::Yilter,
	];

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

/// Character host visual. Child of a [`crate::player::CharacterController`] capsule.
#[derive(Component)]
pub struct PlayerVisual;

/// Independent stampede body. Grid offset from the patch center, in XZ.
#[derive(Component, Debug, Clone, Copy)]
pub struct StampedeMember {
	pub offset: Vec3,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestSetCharacter {
	pub species: CharacterSpecies,
}

/// Spawn every biped and quadruped as its own controller; they share Move / Jump.
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestStampede;

pub(crate) fn apply_set_character(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	requests: Query<(Entity, &RequestSetCharacter)>,
	players: Query<Entity, With<Player>>,
	visuals: Query<Entity, With<PlayerVisual>>,
	herd: Query<Entity, With<StampedeMember>>,
	mut capsules: Query<&mut Visibility, With<PlayerCapsule>>,
) {
	let Ok(player) = players.single() else {
		for (entity, _) in &requests {
			commands.entity(entity).despawn();
		}
		return;
	};

	for (entity, request) in &requests {
		clear_herd(&mut commands, &visuals, &herd, &mut capsules);
		restore_player_controller(&mut commands, player);
		for spawned in spawn_species(&mut commands, request.species, Transform::IDENTITY) {
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

/// Replace the current visual with every biped and quadruped, each on its own capsule.
pub(crate) fn apply_stampede(
	mut commands: Commands,
	mut status: ResMut<GameCommandStatusText>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	store: Res<TerrainEntryStore>,
	requests: Query<Entity, With<RequestStampede>>,
	mut players: Query<(Entity, &mut LinearVelocity), With<Player>>,
	visuals: Query<Entity, With<PlayerVisual>>,
	herd: Query<Entity, With<StampedeMember>>,
	mut capsules: Query<&mut Visibility, With<PlayerCapsule>>,
) {
	let Ok((player, mut velocity)) = players.single_mut() else {
		for entity in &requests {
			commands.entity(entity).despawn();
		}
		return;
	};

	for entity in &requests {
		clear_herd(&mut commands, &visuals, &herd, &mut capsules);
		park_player_for_stampede(&mut commands, player, &mut velocity);
		let count = CharacterSpecies::STAMPEDE.len();
		let center = layout.region_center_xz();
		for (index, species) in CharacterSpecies::STAMPEDE.iter().copied().enumerate() {
			let offset = stampede_offset(index, count);
			let x = center.x + offset.x;
			let z = center.z + offset.z;
			let elevation = store
				.composed_height_at(&layout, x, z)
				.unwrap_or_else(|| base.0.height_at(x, z));
			let body = spawn_character_controller(
				&mut commands,
				controller_spawn_point(x, z, elevation),
			);
			commands.entity(body).insert((
				Name::new(format!("stampede-{}", species.label())),
				StampedeMember { offset },
			));
			if offset.length_squared() < 1e-4 {
				commands.entity(body).insert(CameraFollow);
			}
			for spawned in spawn_species(&mut commands, species, Transform::IDENTITY) {
				commands.entity(spawned).insert((ChildOf(body), PlayerVisual));
			}
		}
		commands.spawn(RequestModeCharacter);
		status.0 = format!(
			"stampede {} species — WASD / jump on every capsule",
			count
		);
		commands.entity(entity).despawn();
	}
}

fn clear_herd(
	commands: &mut Commands,
	visuals: &Query<Entity, With<PlayerVisual>>,
	herd: &Query<Entity, With<StampedeMember>>,
	capsules: &mut Query<&mut Visibility, With<PlayerCapsule>>,
) {
	for member in herd {
		commands.entity(member).try_despawn();
	}
	for visual in visuals {
		commands.entity(visual).try_despawn();
	}
	for mut visibility in capsules.iter_mut() {
		*visibility = Visibility::Hidden;
	}
}

/// Snap herd capsules to sampled ground after layout / mode resets.
pub(crate) fn respawn_stampede_members<F: QueryFilter>(
	layout: &TerrainCellLayout,
	height_at: impl Fn(f32, f32) -> f32,
	members: &mut Query<(&StampedeMember, &mut Transform, &mut LinearVelocity), F>,
) {
	let center = layout.region_center_xz();
	for (member, mut transform, mut velocity) in members {
		let x = center.x + member.offset.x;
		let z = center.z + member.offset.z;
		transform.translation = controller_spawn_point(x, z, height_at(x, z));
		**velocity = Vec3::ZERO;
	}
}

fn stampede_offset(index: usize, count: usize) -> Vec3 {
	let cols = (count as f32).sqrt().ceil().max(1.0) as usize;
	let row = index / cols;
	let col = index % cols;
	let origin = (cols as f32 - 1.0) * 0.5;
	Vec3::new(
		(col as f32 - origin) * STAMPEDE_SPACING,
		0.0,
		(row as f32 - origin) * STAMPEDE_SPACING,
	)
}

/// Walk / run / jump on each visual from its own controller's wish and speed.
pub(crate) fn drive_player_locomotion(
	mut commands: Commands,
	time: Res<Time>,
	controllers: Query<(&LinearVelocity, &MoveWish, Has<Jumping>)>,
	mut visuals: Query<
		(&CharacterMembers, &mut Transform, &ChildOf),
		(With<PlayerVisual>, With<CharacterRoot>),
	>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	let dt = time.delta_secs();
	for (members, mut visual, child_of) in &mut visuals {
		let Ok((velocity, wish, jumping)) = controllers.get(child_of.parent()) else {
			continue;
		};
		let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
		let speed = horizontal.length();
		face_wish(&mut visual, wish.0, dt);
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let clip = locomotion_clip(rig.skeleton, jumping, speed);
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
}

/// Slerp mesh +Z toward camera-relative wish. Coasting keeps the last heading.
fn face_wish(visual: &mut Transform, wish: Vec3, dt: f32) {
	let target = Vec3::new(wish.x, 0.0, wish.z);
	if target.length_squared() < 1e-4 {
		return;
	}
	let target = target.normalize();
	let current = {
		let facing = -visual.forward();
		let xz = Vec3::new(facing.x, 0.0, facing.z);
		if xz.length_squared() < 1e-4 {
			visual.look_to(-target, Vec3::Y);
			return;
		}
		xz.normalize()
	};
	let angle = current.angle_between(target);
	if angle < FACE_DEADZONE {
		return;
	}
	let t = (TURN_RATE * dt / angle).min(1.0);
	visual.look_to(-current.slerp(target, t), Vec3::Y);
}

fn locomotion_clip(skeleton: RigSkeletonKind, jumping: bool, speed: f32) -> AnimClip {
	match skeleton {
		RigSkeletonKind::Humanoid | RigSkeletonKind::Neck => {
			if jumping && speed > RUN_SPEED {
				AnimClip::leap()
			} else if jumping {
				AnimClip::jump()
			} else if speed > RUN_SPEED {
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
			} else if speed > RUN_SPEED {
				AnimClip::gallop()
			} else if speed > WALK_SPEED {
				AnimClip::quadruped_run()
			} else {
				AnimClip::still()
			}
		}
		RigSkeletonKind::Forelimbed => {
			if speed > RUN_SPEED {
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
