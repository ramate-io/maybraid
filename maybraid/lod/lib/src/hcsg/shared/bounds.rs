//! [`HcsgRegions<C>`]: the boxes channel `C` wants values in, and the
//! [`HcsgBounds`] producers that send them.

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{App, IntoScheduleConfigs, Local, Message, MessageWriter, Plugin, Update};

use super::runtime::HcsgSystems;

/// Where channel `C` wants values, as its producer last sent them.
///
/// Generation and presentation on `C` fill and present every value in
/// `boxes`, nearest `focus` first, and retire hosts that touch none of them.
/// An empty set requests nothing and retires everything. A channel has one
/// producer; sources that should share hosts combine into one.
#[derive(Message, Debug)]
pub struct HcsgRegions<C> {
	pub boxes: Vec<Aabb3d>,
	pub focus: Option<Vec3>,
	_channel: PhantomData<fn() -> C>,
}

impl<C> HcsgRegions<C> {
	pub fn new(boxes: Vec<Aabb3d>, focus: Option<Vec3>) -> Self {
		Self { boxes, focus, _channel: PhantomData }
	}
}

impl<C> Clone for HcsgRegions<C> {
	fn clone(&self) -> Self {
		Self::new(self.boxes.clone(), self.focus)
	}
}

impl<C> PartialEq for HcsgRegions<C> {
	fn eq(&self, other: &Self) -> bool {
		self.boxes == other.boxes && self.focus == other.focus
	}
}

/// A frame-side producer for its own channel: a camera, a gameplay region,
/// an explicit warming region.
///
/// [`HcsgBoundsPlugin`] sends [`HcsgRegions<Self>`] whenever the boxes or
/// focus change, so snap them to the cells they cover rather than following
/// a camera exactly.
pub trait HcsgBounds: Send + Sync + 'static {
	type Param: SystemParam + 'static;

	fn regions(param: &SystemParamItem<Self::Param>) -> Vec<Aabb3d>;

	fn focus(param: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		let _ = param;
		None
	}
}

fn produce<B: HcsgBounds>(
	param: StaticSystemParam<B::Param>,
	mut sent: Local<Option<HcsgRegions<B>>>,
	mut regions: MessageWriter<HcsgRegions<B>>,
) {
	let current = HcsgRegions::new(B::regions(&param), B::focus(&param));
	if sent.as_ref() != Some(&current) {
		regions.write(current.clone());
		*sent = Some(current);
	}
}

/// Sends `B`'s regions on channel `B` before [`HcsgSystems`].
pub struct HcsgBoundsPlugin<B>(PhantomData<fn() -> B>);

impl<B> Default for HcsgBoundsPlugin<B> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<B: HcsgBounds> Plugin for HcsgBoundsPlugin<B> {
	fn build(&self, app: &mut App) {
		app.add_message::<HcsgRegions<B>>()
			.add_systems(Update, produce::<B>.before(HcsgSystems));
	}
}
