/// Top-level menu node the playground shell renders via `render_with`.
#[derive(Clone, Debug, PartialEq)]
pub struct Root<T> {
	pub value: T,
}

impl<T> Root<T> {
	pub const fn new(value: T) -> Self {
		Self { value }
	}
}
