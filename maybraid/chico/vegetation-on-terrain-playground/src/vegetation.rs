//! Forest + canopy bump-outs against one composed world source.

use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use chico_forests::{
	CanopyBumpOut, ChicoGrove, ForestIndex, ForestPlugin, ForestPresenter, MediumCanopyBumpOut,
};
use lod::presentation::RegionPresenter;

use crate::bump_out::{
	register_bump_out_lod, CanopyBumpOutPresenter, MediumCanopyBumpOutPresenter,
};

/// Registers grove present and both canopy bump-out channels for world source `S`.
///
/// Terrain near / far / background stay on Durham. Character and urbanization stay out.
/// `S` is the same wrapping used to grow groves (`OnTerrain<H>`, `DevelopmentExclusions<…>`).
/// Nested presenters are named with `'static` so this `Plugin` impl can forward
/// `ForestPlugin::<ForestPresenter<S>>` the same way a monomorphized call site does.
pub struct VegetationPlugin<S> {
	_marker: PhantomData<fn() -> S>,
}

impl<S> Default for VegetationPlugin<S> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<S> Plugin for VegetationPlugin<S>
where
	S: SystemParam + 'static,
	ForestPresenter<'static, 'static, S>: SystemParam + 'static,
	for<'w, 's> <ForestPresenter<'static, 'static, S> as SystemParam>::Item<'w, 's>:
		RegionPresenter<ChicoGrove, ForestIndex>,
	CanopyBumpOutPresenter<'static, 'static, S>: SystemParam + 'static,
	for<'w, 's> <CanopyBumpOutPresenter<'static, 'static, S> as SystemParam>::Item<'w, 's>:
		RegionPresenter<CanopyBumpOut, ForestIndex>,
	MediumCanopyBumpOutPresenter<'static, 'static, S>: SystemParam + 'static,
	for<'w, 's> <MediumCanopyBumpOutPresenter<'static, 'static, S> as SystemParam>::Item<'w, 's>:
		RegionPresenter<MediumCanopyBumpOut, ForestIndex>,
{
	fn build(&self, app: &mut App) {
		app.add_plugins(ForestPlugin::<ForestPresenter<'static, 'static, S>>::default());
		register_bump_out_lod::<
			CanopyBumpOutPresenter<'static, 'static, S>,
			MediumCanopyBumpOutPresenter<'static, 'static, S>,
		>(app);
	}
}
