use crate::CameraFocus;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingleSelect<T> {
	pub value: T,
	pub camera_focus: Option<CameraFocus>,
}

impl<T> SingleSelect<T> {
	pub const fn new(value: T) -> Self {
		Self { value, camera_focus: None }
	}

	pub const fn with_camera_focus(mut self, camera_focus: CameraFocus) -> Self {
		self.camera_focus = Some(camera_focus);
		self
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct VecSelect<T> {
	pub options: Vec<T>,
	pub selected_index: usize,
	pub camera_focus: Option<CameraFocus>,
}

impl<T> VecSelect<T> {
	pub fn new(options: Vec<T>, selected_index: usize) -> Self {
		Self { options, selected_index, camera_focus: None }
	}
}
