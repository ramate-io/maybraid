use crate::{EffectLayer, EffectLayerGroup, EffectRecipe};
use bevy::prelude::*;

#[derive(Clone, Component, crate::EffectRecipe)]
#[effect(name = "parametric", layers = Self::layers)]
struct Parametric<T: Copy + Into<f32> + Send + Sync + 'static> {
	size: T,
}

impl<T: Copy + Into<f32> + Send + Sync + 'static> Parametric<T> {
	fn layers(&self) -> EffectLayerGroup {
		EffectLayer::flipbook_path("vfx/dust/rolling_wave.flipbook.json")
			.with_scale(self.size.into())
			.into_group()
	}
}

#[test]
fn derive_preserves_generic_recipe_parameters() {
	let definition = Parametric { size: 2.5_f32 }.effect_definition();
	assert_eq!(definition.name, "parametric");
	assert_eq!(definition.layers[0].scale, 2.5);
}

#[test]
fn nested_groups_match_composed_transforms_and_add_delays() {
	let local = Transform::from_xyz(2., 1., -1.)
		.with_rotation(Quat::from_rotation_y(0.4))
		.with_scale(Vec3::new(1., 2., 3.));
	let rotation = Quat::from_rotation_z(0.7);
	let offset = Vec3::new(7., -2., 1.);
	let layer = EffectLayer::flipbook_path("test.flipbook.json")
		.with_transform(local)
		.with_scale(0.5)
		.with_delay(0.2);
	let group = EffectLayerGroup::from_groups([layer.into_group().delayed(0.3)])
		.scaled(-2.)
		.rotated(rotation)
		.translated(offset)
		.delayed(0.4);
	let layers: Vec<_> = group.into_iter().collect();
	assert!((layers[0].delay - 0.9).abs() < 1e-6);
	let parent = Transform::from_translation(offset)
		.with_rotation(rotation)
		.with_scale(Vec3::splat(-2.));
	let expected =
		parent.compute_affine() * Transform { scale: local.scale * 0.5, ..local }.compute_affine();
	let actual =
		Transform { scale: layers[0].transform.scale * layers[0].scale, ..layers[0].transform }
			.compute_affine();
	assert!(actual.abs_diff_eq(expected, 1e-5));
}

#[test]
fn placement_helpers_preserve_other_axes() {
	let scale = Vec3::new(2., 3., 4.);
	let rotation = Quat::from_rotation_x(0.4);
	let layer = EffectLayer::flipbook_path("test.flipbook.json")
		.with_transform(Transform::from_scale(scale))
		.with_translation(Vec3::Y)
		.with_rotation(rotation);
	assert_eq!(layer.transform.scale, scale);
	assert_eq!(layer.transform.translation, Vec3::Y);
	assert_eq!(layer.transform.rotation, rotation);
}
