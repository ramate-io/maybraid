//! Messages from legacy frame-synchronous present drains (still used by
//! [`super::runtime`] until that path is removed).

use std::marker::PhantomData;

use bevy::prelude::*;

use crate::gen::Id;

/// One origin id inserted by a present-side generate drain.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodGenerated<T: Send + Sync + 'static> {
	pub id: Id,
	pub _marker: PhantomData<T>,
}

impl<T: Send + Sync + 'static> LodGenerated<T> {
	pub fn new(id: Id) -> Self {
		Self { id, _marker: PhantomData }
	}
}
