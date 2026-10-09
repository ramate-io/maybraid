//! [`HcsgRegions<C>`]: the boxes channel `C` wants values in, and the
//! [`HcsgBounds`] producers that send them.

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{App, IntoScheduleConfigs, Local, Message, MessageWriter, Plugin, Update};

use super::runtime::HcsgSystems;

/// Coarse scheduling weight for a channel. The worker spends quanta in
/// proportion to [`Self::weight`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HcsgClass {
	/// Near terrain, groves, bump-outs, developments, furniture, mobs.
	Near,
	/// Far terrain.
	Far,
	/// Background terrain.
	Background,
	/// Language tiles and the naming window.
	Ambient,
}

impl HcsgClass {
	pub const fn weight(self) -> u32 {
		match self {
			Self::Near => 8,
			Self::Far => 4,
			Self::Background => 2,
			Self::Ambient => 1,
		}
	}
}

/// Where channel `C` wants values, as its producer last sent them.
///
/// Generation and presentation on `C` fill and present every value in
/// `boxes`, nearest `focus` first, and retire hosts that touch none of them.
/// An empty set requests nothing and retires everything. A channel has one
/// producer; sources that should share hosts combine into one.
///
/// `focus` only orders ids inside one subscription. `class` is the channel's
/// scheduling weight. Neither participates in change detection: producers
/// resend only when the boxes change.
#[derive(Message, Debug)]
pub struct HcsgRegions<C> {
	pub boxes: Vec<Aabb3d>,
	pub focus: Option<Vec3>,
	pub class: HcsgClass,
	_channel: PhantomData<fn() -> C>,
}

impl<C> HcsgRegions<C> {
	pub fn new(boxes: Vec<Aabb3d>, focus: Option<Vec3>, class: HcsgClass) -> Self {
		Self { boxes, focus, class, _channel: PhantomData }
	}
}

impl<C> Clone for HcsgRegions<C> {
	fn clone(&self) -> Self {
		Self::new(self.boxes.clone(), self.focus, self.class)
	}
}

/// A frame-side producer for its own channel: a camera, a gameplay region,
/// an explicit warming region.
///
/// [`HcsgBoundsPlugin`] sends [`HcsgRegions<Self>`] whenever the boxes
/// change, so snap them to the cells they cover rather than following a
/// camera exactly. The focus rides along with the next send.
pub trait HcsgBounds: Send + Sync + 'static {
	/// Scheduling class stamped on every [`HcsgRegions`] this producer sends.
	const CLASS: HcsgClass;

	type Param: SystemParam + 'static;

	fn regions(param: &SystemParamItem<Self::Param>) -> Vec<Aabb3d>;

	fn focus(param: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		let _ = param;
		None
	}
}

/// Whether a [`Gated`] producer sends its regions.
pub trait HcsgGate: Send + Sync + 'static {
	type Param: SystemParam + 'static;

	fn open(param: &SystemParamItem<Self::Param>) -> bool;
}

/// `B`'s regions while `G` is open, and none (retiring every host) while it
/// is closed.
pub struct Gated<G, B>(PhantomData<fn() -> (G, B)>);

impl<G: HcsgGate, B: HcsgBounds> HcsgBounds for Gated<G, B> {
	const CLASS: HcsgClass = B::CLASS;

	type Param = (G::Param, B::Param);

	fn regions(param: &SystemParamItem<Self::Param>) -> Vec<Aabb3d> {
		if G::open(&param.0) {
			B::regions(&param.1)
		} else {
			Vec::new()
		}
	}

	fn focus(param: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		B::focus(&param.1)
	}
}

fn produce<B: HcsgBounds>(
	param: StaticSystemParam<B::Param>,
	mut sent: Local<Option<Vec<Aabb3d>>>,
	mut regions: MessageWriter<HcsgRegions<B>>,
) {
	let boxes = B::regions(&param);
	if sent.as_ref() != Some(&boxes) {
		regions.write(HcsgRegions::new(boxes.clone(), B::focus(&param), B::CLASS));
		*sent = Some(boxes);
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
