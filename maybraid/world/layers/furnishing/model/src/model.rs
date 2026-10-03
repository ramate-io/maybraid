//! [`Furnishing`] wrapper. Nothing wraps it, so it does not implement [`TerrainModel`].

use std::marker::PhantomData;

/// Model `F` after furnishing. A sibling over urbanization, not a ground.
pub struct Furnishing<F>(PhantomData<fn() -> F>);
