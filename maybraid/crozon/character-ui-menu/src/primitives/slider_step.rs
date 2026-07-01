use super::slider::Slider;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderStep<E> {
	pub slider: Slider,
	pub decrease: E,
	pub increase: E,
}

impl<E> SliderStep<E> {
	pub const fn new(slider: Slider, decrease: E, increase: E) -> Self {
		Self { slider, decrease, increase }
	}
}
