use bevy::prelude::*;
use character_ui_menu::{LabelOption, ListValues, SingleSelect};

use crate::render::select_picker::{SelectEventMap, SelectPicker};
use crate::render::{MenuThumbnailContext, RenderContext, RenderMenu, Renderer};

pub trait SectionMenuMap<T: Copy> {
	fn render_menu_for<C: MenuThumbnailContext>(
		&self,
		value: T,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	);
}

/// Species-style picker that swaps between one menu subtree per selected value.
pub struct SectionSelect<'a, E, T, M, S>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SelectEventMap<E, T> + Copy,
	S: SectionMenuMap<T> + ?Sized,
{
	pub select: SingleSelect<T>,
	pub picker: M,
	pub menus: &'a S,
	_marker: core::marker::PhantomData<E>,
}

impl<'a, E, T, M, S> SectionSelect<'a, E, T, M, S>
where
	E: Copy + Send + Sync + 'static,
	T: Copy,
	M: SelectEventMap<E, T> + Copy,
	S: SectionMenuMap<T> + ?Sized,
{
	pub const fn new(select: SingleSelect<T>, picker: M, menus: &'a S) -> Self {
		Self { select, picker, menus, _marker: core::marker::PhantomData }
	}
}

impl<'a, E, T, M, S> RenderMenu for SectionSelect<'a, E, T, M, S>
where
	E: Copy + Send + Sync + 'static,
	T: Copy + PartialEq + LabelOption + ListValues,
	M: SelectEventMap<E, T> + Copy,
	S: SectionMenuMap<T> + ?Sized,
{
	fn render_with<C: MenuThumbnailContext>(
		&self,
		renderer: &Renderer,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		character_ui_menu::Labeled::new(
			"Species",
			SelectPicker::new(self.select, self.picker),
		)
		.render_with(renderer, parent, context);
		self.menus.render_menu_for(self.select.value, renderer, parent, context);
	}
}
