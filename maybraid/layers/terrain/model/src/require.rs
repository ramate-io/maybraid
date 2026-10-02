//! [`RequireLayer`]: fail loudly when a layer's dependencies are missing.

use std::any::type_name;

use bevy::app::{App, Plugin};

/// Layer plugins call this from [`Plugin::finish`] instead of adding other layers.
pub trait RequireLayer {
	/// Panic naming `P` and the requiring plugin `By` when `P` was never added.
	fn require_layer<P: Plugin, By: ?Sized>(&self);
}

impl RequireLayer for App {
	fn require_layer<P: Plugin, By: ?Sized>(&self) {
		if !self.is_plugin_added::<P>() {
			panic!(
				"{} requires {}; add it to the app before running",
				type_name::<By>(),
				type_name::<P>()
			);
		}
	}
}
