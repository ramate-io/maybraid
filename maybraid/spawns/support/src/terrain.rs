//! Published terrain lookup shared by actor support adapters.

use bevy::prelude::*;
use durham::{BaseTerrainNoise, DurhamSurface};
use lod::hcsg::HcsgStorage;
use richmond::PaddedTerrain;

/// Prefer published building pads; a busy store leaves the request pending.
/// Durham's composed surface supplies terrain outside published padded cells.
pub fn terrain_height_at(
	surface: &DurhamSurface,
	storage: Option<&HcsgStorage>,
	xz: Vec2,
) -> Option<f32> {
	type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
	if let Some(storage) = storage {
		match PaddedTerrain::<Ground>::published_height_at(storage, xz.x, xz.y) {
			Ok(Some(height)) => return Some(height),
			Ok(None) => {}
			Err(_) => return None,
		}
	}
	surface.height_at(xz).ok().flatten()
}

/// Temporary elevation while composed terrain has not been published.
pub fn holding_elevation(base: &BaseTerrainNoise, x: f32, z: f32) -> f32 {
	base.height_at(x, z) + base.height_scale * 0.35
}

/// Generated search hint; only collider geometry can accept the standing pose.
pub fn surface_or_hold_at(surface: &DurhamSurface, xz: Vec2) -> f32 {
	surface
		.composed_height_at(xz)
		.unwrap_or_else(|| holding_elevation(surface.base(), xz.x, xz.y))
}

#[cfg(test)]
mod tests;
