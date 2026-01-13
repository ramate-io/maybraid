use bevy::prelude::*;
use comproc::noise::config::NoiseConfig;
use noise::{NoiseFn, Seedable};
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait NoiseDispatchItem: Component + Send + Sync + Debug {
	fn from_noise_dispatch_value(value: f32) -> Self;

	fn spawn_noise_dispatched_item(&self, commands: &mut Commands, position: Vec3) -> Entity;
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
	min: Vec2,
	max: Vec2,
}

#[derive(Component, Clone)]
pub struct DispatchNoiseSkillMap<T: NoiseDispatchItem, N: NoiseFn<f64, 2> + Seedable> {
	__marker: PhantomData<T>,
	noise: NoiseConfig<2, N>,
	extents: NoiseSkillMapExtents,
	steps: u32,
}
