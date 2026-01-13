use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use comproc::noise::config::NoiseConfig;
use noise::Perlin;
use skill_map::{
	maps::noise_dispatch::{DispatchNoiseSkillMap, NoiseDispatchItem, NoiseSkillMapExtents},
	viewport::SkillMapViewportId,
	SkillMapId,
};

#[derive(Component, Debug, Clone)]
pub struct DemoNoiseDispatchItem {
	value: f32,
}

impl NoiseDispatchItem for DemoNoiseDispatchItem {
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
		// Spawn a colored square sprite the width and height of one extent step and the color based on the value
		let width = extents.max.x - extents.min.x;
		let height = extents.max.y - extents.min.y;
		let color = Color::srgb(self.value, self.value, self.value);

		let entity = commands.spawn((
			Sprite { custom_size: Some(Vec2::new(width, height)), color: color, ..default() },
			Transform::from_translation(position),
			render_layer,
		));

		entity.id()
	}
}

pub fn skill_map_playground(mut commands: Commands) {
	log::info!("Spawning skill map playground");

	commands.spawn((
		SkillMapId(0),
		SkillMapViewportId(0),
		DispatchNoiseSkillMap::<DemoNoiseDispatchItem, Perlin>::new(
			NoiseConfig::<2, Perlin>::new(Perlin::new(0)),
			NoiseSkillMapExtents::default(),
		),
	));
}
