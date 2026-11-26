//! Exercise expansion from a separate crate, including a where clause.
use maybraid_vfx::{EffectLayer, EffectRecipe};

#[derive(EffectRecipe)]
#[effect(name = "external", layers = Self::layers)]
struct External<T>
where
	T: Copy + Into<f32>,
{
	size: T,
}

impl<T: Copy + Into<f32>> External<T> {
	fn layers(&self) -> [EffectLayer; 1] {
		[EffectLayer::flipbook_path("vfx/dust/rolling_wave.flipbook.json")
			.with_scale(self.size.into())]
	}
}

#[test]
fn derives_without_importing_definition_at_the_call_site() {
	let effect = External { size: 0.8_f32 }.effect_definition();
	assert_eq!(effect.name, "external");
	assert_eq!(effect.layers[0].scale, 0.8);
}
