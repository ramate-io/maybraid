use crate::CameraFocus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentifiedAsset {
	pub id: &'static str,
	pub label: &'static str,
	pub path: &'static str,
}

impl IdentifiedAsset {
	pub const fn new(id: &'static str, label: &'static str, path: &'static str) -> Self {
		Self { id, label, path }
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetSingleSelect<T> {
	pub value: T,
	pub camera_focus: Option<CameraFocus>,
}

impl<T> AssetSingleSelect<T> {
	pub const fn new(value: T) -> Self {
		Self { value, camera_focus: None }
	}

	pub const fn with_camera_focus(mut self, camera_focus: CameraFocus) -> Self {
		self.camera_focus = Some(camera_focus);
		self
	}
}
