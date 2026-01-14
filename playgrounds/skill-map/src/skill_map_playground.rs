use bevy::camera::visibility::RenderLayers;
use bevy::math::bounding::Aabb2d;
use bevy::prelude::*;
use comproc::noise::config::NoiseConfig;
use noise::Perlin;
use skill_map::{
	interaction::{CollisionLayer, LeftCollidable, RightCollidable, RightCollider},
	maps::noise_dispatch::{
		DispatchNoiseSkillMap, NoiseDispatchItem, NoiseSkillMapExtents, NoiseSkillMapPlugin,
	},
	viewport::SkillMapViewportId,
	SkillMapId, SkillMapPlugin,
};

pub struct SkillMapPlaygroundPlugin;

impl Plugin for SkillMapPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(SkillMapPlugin::default());
		app.add_plugins(NoiseSkillMapPlugin::<Tile, Perlin>::default());
		app.add_systems(Update, skill_map_playground.run_if(run_once));
	}
}

pub fn skill_map_playground(mut commands: Commands) {
	log::info!("Spawning skill map playground");

	commands.spawn((
		SkillMapId(0),
		SkillMapViewportId(0),
		Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
		DispatchNoiseSkillMap::<Tile, Perlin>::new(
			NoiseConfig::<2, Perlin>::new(Perlin::new(0)),
			NoiseSkillMapExtents::default(),
		),
	));
}

#[derive(Component, Debug, Clone)]
pub struct InteractionLayer;

impl CollisionLayer for InteractionLayer {}

#[derive(Component, Debug, Clone, Default)]
pub struct Water {}

impl RightCollidable for Water {
	fn spawn_right_collision_entity(&self, commands: &mut Commands, _left: Entity) -> Entity {
		commands.spawn(()).id()
	}
}

impl NoiseDispatchItem for Water {
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
		let width = extents.max.x - extents.min.x;
		let height = extents.max.y - extents.min.y;

		// color is blue
		let color = Color::srgb(0.0, 0.0, 1.0);

		let entity = commands.spawn((
			Sprite { custom_size: Some(Vec2::new(width, height)), color: color, ..default() },
			Transform::from_translation(position),
			render_layer,
			RightCollider::new(self.clone(), Aabb2d::new(extents.center(), extents.half_size())),
			InteractionLayer,
		));

		log::info!("Spawned noise dispatched item at {:?}", position);

		entity.id()
	}
}

#[derive(Component, Debug, Clone, Default)]
pub struct Land {}

impl RightCollidable for Land {
	fn spawn_right_collision_entity(&self, commands: &mut Commands, _right: Entity) -> Entity {
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
		let width = extents.max.x - extents.min.x;
		let height = extents.max.y - extents.min.y;

		// color is brown
		let color = Color::srgb(0.5, 0.25, 0.0);

		let entity = commands.spawn((
			Sprite { custom_size: Some(Vec2::new(width, height)), color: color, ..default() },
			Transform::from_translation(position),
			render_layer,
			RightCollider::new(self.clone(), Aabb2d::new(extents.center(), extents.half_size())),
			InteractionLayer,
		));

		log::info!("Spawned noise dispatched item at {:?}", position);

		entity.id()
	}
}

#[derive(Component, Debug, Clone, Default)]
pub struct PowerUp {}

impl RightCollidable for PowerUp {
	fn spawn_right_collision_entity(&self, commands: &mut Commands, _left: Entity) -> Entity {
		commands.spawn(()).id()
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
		let width = extents.max.x - extents.min.x;
		let height = extents.max.y - extents.min.y;

		// color is purple
		let color = Color::srgb(1.0, 0.0, 1.0);

		let entity = commands.spawn((
			Sprite { custom_size: Some(Vec2::new(width, height)), color: color, ..default() },
			Transform::from_translation(position),
			render_layer,
			RightCollider::new(self.clone(), Aabb2d::new(extents.center(), extents.half_size())),
			InteractionLayer,
		));

		log::info!("Spawned noise dispatched item at {:?}", position);

		entity.id()
	}
}

#[derive(Component, Debug, Clone)]
pub enum Tile {
	Water(Water),
	Land(Land),
	PowerUp(PowerUp),
}

impl RightCollidable for Tile {
	fn spawn_right_collision_entity(&self, commands: &mut Commands, _left: Entity) -> Entity {
		match self {
			Tile::Water(water) => water.spawn_right_collision_entity(commands, _left),
			Tile::Land(land) => land.spawn_right_collision_entity(commands, _left),
			Tile::PowerUp(power_up) => power_up.spawn_right_collision_entity(commands, _left),
		}
	}
}

impl NoiseDispatchItem for Tile {
	fn from_noise_dispatch_value(value: f32) -> Self {
		if value < -0.5 {
			Tile::Water(Water::default())
		} else if value > 0.8 {
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
