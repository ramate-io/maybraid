//! Wrapper types for disk-backed assets and caches (see legacy [`render_item`](https://github.com/ramate-io/maybraid/tree/main/util/render-item) `mesh::cache`).
//!
//! New code should prefer explicit composition: wrap your [`crate::dispatch::RenderItem`] payload
//! in these types and delegate in your own `RenderItem` impl until disk/cache helpers move here.

use std::marker::PhantomData;

/// Placeholder for “fetch or build from disk” layering over an inner render payload `T`.
#[derive(Clone, Debug)]
pub struct DiskBacked<T> {
	pub inner: T,
	_marker: PhantomData<()>,
}

impl<T> DiskBacked<T> {
	pub fn new(inner: T) -> Self {
		Self { inner, _marker: PhantomData }
	}
}

/// Placeholder for mesh-handle / mesh-body caches keyed by an author-chosen key type `K`.
#[derive(Clone, Debug)]
pub struct MeshCacheLayer<T, K> {
	pub inner: T,
	_marker: PhantomData<K>,
}

impl<T, K> MeshCacheLayer<T, K> {
	pub fn new(inner: T) -> Self {
		Self { inner, _marker: PhantomData }
	}
}
