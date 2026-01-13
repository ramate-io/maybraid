use crate::SkillMapRenderLayer;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use comproc::noise::config::NoiseConfig;
use noise::{NoiseFn, Seedable};
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait NoiseDispatchItem: Component + Send + Sync + Debug {
	fn from_noise_dispatch_value(value: f32) -> Self;

	fn spawn_noise_dispatched_item(
		&self,
		commands: &mut Commands,
		position: Vec3,
		render_layer: RenderLayers,
		extents: &NoiseSkillMapExtents,
	) -> Entity;
}

#[derive(Component, Clone)]
pub struct NoiseDispatch<T: NoiseDispatchItem> {
	item: T,
}

impl<T: NoiseDispatchItem> NoiseDispatch<T> {
	pub fn new(item: T) -> Self {
		Self { item }
	}

	pub fn item(&self) -> &T {
		&self.item
	}
}

#[derive(Component, Clone)]
pub struct NoiseSkillMapExtents {
	pub min: Vec2,
	pub max: Vec2,
	pub steps: u32,
}

impl NoiseSkillMapExtents {
	pub fn width(&self) -> f32 {
		self.max.x - self.min.x
	}

	pub fn height(&self) -> f32 {
		self.max.y - self.min.y
	}

	pub fn x_step_size(&self) -> f32 {
		self.width() / self.steps as f32
	}
	pub fn y_step_size(&self) -> f32 {
		self.height() / self.steps as f32
	}
}

impl Default for NoiseSkillMapExtents {
	fn default() -> Self {
		// Typical skill map will be a kilometer with 100 steps
		Self { min: Vec2::new(-1000.0, -1000.0), max: Vec2::new(1000.0, 1000.0), steps: 100 }
	}
}

#[derive(Component, Clone)]
pub struct DispatchNoiseSkillMap<T: NoiseDispatchItem, N: NoiseFn<f64, 2> + Seedable + Send + Sync>
{
	__marker: PhantomData<T>,
	noise: NoiseConfig<2, N>,
	extents: NoiseSkillMapExtents,
}

impl<T: NoiseDispatchItem, N: NoiseFn<f64, 2> + Seedable + Send + Sync>
	DispatchNoiseSkillMap<T, N>
{
	pub fn new(noise: NoiseConfig<2, N>, extents: NoiseSkillMapExtents) -> Self {
		Self { __marker: PhantomData, noise, extents }
	}

	fn spawn_noise_skill_map(
		&self,
		commands: &mut Commands,
		_position: Vec3, // TODO: we may translate the extens by this value in the future
		render_layer: RenderLayers,
	) -> Vec<Entity> {
		let mut entities = Vec::new();
		let height = self.extents.max.y - self.extents.min.y;
		let width = self.extents.max.x - self.extents.min.x;
		let min_x = self.extents.min.x;
		let min_y = self.extents.min.y;
		let y_step_size = height / self.extents.steps as f32;
		let x_step_size = width / self.extents.steps as f32;

		for x_step in 0..self.extents.steps {
			for y_step in 0..self.extents.steps {
				let position = Vec2::new(
					min_x + x_step as f32 * x_step_size,
					min_y + y_step as f32 * y_step_size,
				);
				let value = self.noise.vec2_freqo(position) as f32;
				let item = T::from_noise_dispatch_value(value);
				let entity = item.spawn_noise_dispatched_item(
					commands,
					position.extend(0.0),
					render_layer.clone(),
					&self.extents,
				);
				entities.push(entity);
			}
		}
		entities
	}
}

#[derive(Debug, Clone, Default)]
pub struct NoiseSkillMapPlugin<T: NoiseDispatchItem, N: NoiseFn<f64, 2> + Seedable + Send + Sync> {
	__marker: PhantomData<(T, N)>,
}

impl<T: NoiseDispatchItem, N: NoiseFn<f64, 2> + Seedable + Send + Sync + 'static>
	NoiseSkillMapPlugin<T, N>
{
	pub fn spawn_noise_skill_map(
		mut commands: Commands,
		query: Query<
			(&DispatchNoiseSkillMap<T, N>, &Transform, &SkillMapRenderLayer),
			Added<SkillMapRenderLayer>,
		>,
	) {
		for (noise_skill_map, transform, render_layer) in &query {
			noise_skill_map.spawn_noise_skill_map(
				&mut commands,
				transform.translation,
				render_layer.0.clone(),
			);
		}
	}
}
