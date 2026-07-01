use super::single_select::SingleSelect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cycle<E, T> {
	pub select: SingleSelect<T>,
	pub minus: E,
	pub plus: E,
}

impl<E, T> Cycle<E, T> {
	pub const fn new(select: SingleSelect<T>, minus: E, plus: E) -> Self {
		Self { select, minus, plus }
	}
}
