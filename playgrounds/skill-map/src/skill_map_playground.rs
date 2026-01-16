pub mod fireball;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use comproc::noise::config::NoiseConfig;
use fireball::{DispatchCameraFireball, Fireball, FireballPlugin};
use noise::{Fbm, OpenSimplex};
use skill_map::{
	interaction::{
		CollisionLayer, CollisionPlugin, LeftCollidable, LeftCollider, RightCollidable,
		RightCollider,
	},
	maps::noise_dispatch::{
		DispatchNoiseSkillMap, NoiseDispatchItem, NoiseSkillMapExtents, NoiseSkillMapPlugin,
	},
	viewport::{Debraid, SkillMapViewportId, TrackCameraTransform},
	SkillMapId, SkillMapPlugin, SkillMapRenderTarget,
};

pub struct SkillMapPlaygroundPlugin;

impl Plugin for SkillMapPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(SkillMapPlugin::default());
		app.add_plugins(NoiseSkillMapPlugin::<Tile, Fbm<OpenSimplex>>::default());
		app.add_plugins(CollisionPlugin::<Player, PowerUp, InteractionLayer>::default());
		app.add_plugins(CollisionPlugin::<Player, Water, InteractionLayer>::default());
		app.add_plugins(FireballPlugin);
		app.add_systems(Update, skill_map_playground.run_if(run_once));
	}
}

pub fn skill_map_playground(mut commands: Commands) {
	log::info!("Spawning skill map playground");

	commands.spawn((
		SkillMapId(0),
		SkillMapViewportId(0),
		Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
		DispatchNoiseSkillMap::<Tile, Fbm<OpenSimplex>>::new(
			NoiseConfig::<2, Fbm<OpenSimplex>>::new(Fbm::new(0)).with_frequency(0.1),
			NoiseSkillMapExtents::default(),
		),
	));

	// add a white dot to track the camera transform
	let id = commands
		.spawn((
			SkillMapId(0),
			SkillMapViewportId(0),
			SkillMapRenderTarget,
			TrackCameraTransform,
			Sprite { custom_size: Some(Vec2::new(10.0, 10.0)), color: Color::WHITE, ..default() },
			LeftCollider::new(Player::default(), Vec2::new(10.0, 10.0)),
			InteractionLayer,
			Transform::from_translation(Vec3::new(0.0, 0.0, 0.001)),
		))
		.id();

	log::info!("Track camera transform entity: {:?}", id);
}

#[derive(Component, Debug, Clone)]
pub struct InteractionLayer;

impl CollisionLayer for InteractionLayer {}

#[derive(Component, Debug, Clone, Default)]
pub struct Player;

impl LeftCollidable for Player {
	fn spawn_left_collision_entity(
		&self,
		commands: &mut Commands,
		_left: Entity,
		_right: Entity,
	) -> Entity {
		commands.spawn(()).id()
	}
}

#[derive(Component, Debug, Clone, Default)]
pub struct Water {
	value: f32,
}

impl Water {
	pub fn new(value: f32) -> Self {
		Self { value }
	}

	pub fn value(&self) -> f32 {
		self.value
	}
}

impl RightCollidable for Water {
	fn spawn_right_collision_entity(
		&self,
		commands: &mut Commands,
		_left: Entity,
		_right: Entity,
	) -> Entity {
		log::info!("Spawning right collision entity for water");
		commands.spawn(()).id()
	}
}

impl NoiseDispatchItem for Water {
	fn from_noise_dispatch_value(value: f32) -> Self {
		Self { value }
	}

	fn spawn_noise_dispatched_item(
		&self,
		commands: &mut Commands,
		position: Vec3,
		render_layer: RenderLayers,
		extents: &NoiseSkillMapExtents,
	) -> Entity {
		let width = extents.x_step_size();
		let height = extents.y_step_size();

		// color is blue
		let color = Color::srgb(0.0, 0.0, 1.0);

		let entity = commands
			.spawn((
				Sprite { custom_size: Some(Vec2::new(width, height)), color: color, ..default() },
				Transform::from_translation(position),
				render_layer.clone(),
				RightCollider::new(self.clone(), Vec2::new(width, height)),
				InteractionLayer,
			))
			.id();

		entity
	}
}

#[derive(Component, Debug, Clone, Default)]
pub struct Land {}

impl RightCollidable for Land {
	fn spawn_right_collision_entity(
		&self,
		commands: &mut Commands,
		_left: Entity,
		_right: Entity,
	) -> Entity {
		commands.spawn(()).id()
	}
}

impl NoiseDispatchItem for Land {
	fn from_noise_dispatch_value(_value: f32) -> Self {
		Self::default()
	}

	fn spawn_noise_dispatched_item(
		&self,
		commands: &mut Commands,
		position: Vec3,
		render_layer: RenderLayers,
		extents: &NoiseSkillMapExtents,
	) -> Entity {
		let width = extents.x_step_size();
		let height = extents.y_step_size();

		// color is brown
		let color = Color::srgb(0.5, 0.25, 0.0);

		let entity = commands
			.spawn((
				Sprite { custom_size: Some(Vec2::new(width, height)), color: color, ..default() },
				Transform::from_translation(position),
				render_layer,
			))
			.id();

		// todo: debug why spawning alongside sprite fails
		let _collision_entity = commands
			.spawn((RightCollider::new(self.clone(), Vec2::new(width, height)), InteractionLayer));

		entity
	}
}

#[derive(Component, Debug, Clone, Default)]
pub struct PowerUp {}

impl RightCollidable for PowerUp {
	fn spawn_right_collision_entity(
		&self,
		commands: &mut Commands,
		_left: Entity,
		right: Entity,
	) -> Entity {
		log::info!("Spawning power up");

		commands.entity(right).despawn();
		commands
			.spawn((
				DispatchCameraFireball(Fireball::new(
					5.0, // 5 seconds
					1.0,
					0.0,                       // 0.25 seconds,
					Vec3::new(0.0, 30.0, 0.0), // 30 meters per second,
					0.0,
				)),
				InteractionLayer,
			))
			.id()
	}
}

impl NoiseDispatchItem for PowerUp {
	fn from_noise_dispatch_value(_value: f32) -> Self {
		Self::default()
	}

	fn spawn_noise_dispatched_item(
		&self,
		commands: &mut Commands,
		position: Vec3,
		render_layer: RenderLayers,
		extents: &NoiseSkillMapExtents,
	) -> Entity {
		// spawn land underneath
		Land::from_noise_dispatch_value(0.0).spawn_noise_dispatched_item(
			commands,
			position,
			render_layer.clone(),
			extents,
		);

		let width = extents.x_step_size();
		let height = extents.y_step_size();

		// color is purple
		let color = Color::srgb(1.0, 0.0, 1.0);

		let mut transform = Transform::from_translation(position);
		transform.translation.z = 0.002;

		let entity = commands
			.spawn((
				Sprite { custom_size: Some(Vec2::new(width, height)), color: color, ..default() },
				transform.clone(),
				render_layer,
				RightCollider::new(self.clone(), Vec2::new(width, height)),
				InteractionLayer,
			))
			.id();

		entity
	}
}

#[derive(Component, Debug, Clone)]
pub enum Tile {
	Water(Water),
	Land(Land),
	PowerUp(PowerUp),
}

impl RightCollidable for Tile {
	fn spawn_right_collision_entity(
		&self,
		commands: &mut Commands,
		_left: Entity,
		_right: Entity,
	) -> Entity {
		match self {
			Tile::Water(water) => water.spawn_right_collision_entity(commands, _left, _right),
			Tile::Land(land) => land.spawn_right_collision_entity(commands, _left, _right),
			Tile::PowerUp(power_up) => {
				power_up.spawn_right_collision_entity(commands, _left, _right)
			}
		}
	}
}

impl NoiseDispatchItem for Tile {
	fn from_noise_dispatch_value(value: f32) -> Self {
		if value < -0.1 {
			Tile::Water(Water::from_noise_dispatch_value(value))
		} else if value > 0.1 {
			Tile::PowerUp(PowerUp::default())
		} else {
			Tile::Land(Land::default())
		}
	}

	fn spawn_noise_dispatched_item(
		&self,
		commands: &mut Commands,
		position: Vec3,
		render_layer: RenderLayers,
		extents: &NoiseSkillMapExtents,
	) -> Entity {
		match self {
			Tile::Water(water) => {
				water.spawn_noise_dispatched_item(commands, position, render_layer, extents)
			}
			Tile::Land(land) => {
				land.spawn_noise_dispatched_item(commands, position, render_layer, extents)
			}
			Tile::PowerUp(power_up) => {
				power_up.spawn_noise_dispatched_item(commands, position, render_layer, extents)
			}
		}
	}
}
