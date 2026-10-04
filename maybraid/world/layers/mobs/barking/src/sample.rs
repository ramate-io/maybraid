//! What Barking samples from the layers under it.
//!
//! The traits live here. [`Vegetation`] and [`chico::Chico`] never see them.
//! [`MobWorldSample`] and [`MobWorldHosts`] stay the group-generation seam;
//! these traits are how the index is filled.

use bevy::ecs::system::{ReadOnlySystemParam, ResMut, SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3, Vec3Swizzles};
use bevy::prelude::{GlobalTransform, Query, Res};
use chico::{select_cell, Chico, ForestExtent, ForestIndex, LayeringKind};
use procedural_common::NoiseParams;
use richmond::layer::RichmondRead;
use richmond::{DiscoverablePlace, Richmond, RichmondGround};
use terrain_layer_model::TerrainModel;
use urbanization_cells::{select_kind, UrbanizationExtent, UrbanizationIndex, UrbanizationKind};
use urbanization_layer_model::{UrbanRead, UrbanSetting, Urbanization};

use crate::generation::MobPlantHost;
use crate::index::urban_leaf_arrival_radius;

/// Layering at a point. Implemented for [`Chico<U>`].
pub trait ForestSelection: Send + Sync + 'static {
	type Read: ReadOnlySystemParam + 'static;

	fn pick(read: &SystemParamItem<'_, '_, Self::Read>) -> (NoiseParams, Option<LayeringKind>);

	/// Occupied layers at `xz`, in `0..=4`.
	fn layers_at(noise: NoiseParams, layering: Option<LayeringKind>, xz: Vec2) -> u8;
}

/// Noise and an optional pinned urbanization kind.
pub trait UrbanSelection: TerrainModel {
	fn selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>);

	fn kind_at(noise: NoiseParams, pinned: Option<UrbanizationKind>, xz: Vec2) -> UrbanizationKind;
}

/// Plant anchors taken from urbanization leaves and development cells.
pub trait PlantHosts: TerrainModel {
	fn plant_hosts(read: &SystemParamItem<'_, '_, Self::Read>, region: Aabb3d)
		-> Vec<MobPlantHost>;
}

/// Presented settings and discoverable places, in that order.
pub trait DiscoverablePlaces: Send + Sync + 'static {
	type Places: SystemParam + 'static;

	fn places(read: &SystemParamItem<'_, '_, Self::Places>) -> Vec<MobPlantHost>;
}

/// #720 cross-layer write: select urbanization cells the mob keep covers.
///
/// Generation continuations retire this. It does not sit on [`Urbanization`].
pub trait SelectUrbanization: Send + Sync + 'static {
	type Select: SystemParam + 'static;

	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d);
}

/// Ground Barking's vegetation impl can sample.
pub trait BarkingEnvironment:
	TerrainModel + PlantHosts + UrbanSelection + DiscoverablePlaces + SelectUrbanization
{
}

impl<T> BarkingEnvironment for T where
	T: TerrainModel + PlantHosts + UrbanSelection + DiscoverablePlaces + SelectUrbanization
{
}

pub(crate) fn chico_layers_at(noise: NoiseParams, layering: Option<LayeringKind>, xz: Vec2) -> u8 {
	let position = Vec3::new(xz.x, 0.0, xz.y);
	let (ix, iz) = ForestExtent::cell_index_containing(position);
	let extent = ForestExtent::from_cell_index(ix, iz);
	let layers = match layering {
		Some(kind) => kind.layering().typical_layers(),
		None => select_cell(extent, noise),
	};
	[layers.tufts, layers.understory, layers.lower_canopy, layers.upper_canopy]
		.into_iter()
		.filter(Option::is_some)
		.count() as u8
}

pub(crate) fn richmond_kind_at(
	noise: NoiseParams,
	pinned: Option<UrbanizationKind>,
	xz: Vec2,
) -> UrbanizationKind {
	if let Some(kind) = pinned {
		return kind;
	}
	let position = Vec3::new(xz.x, 0.0, xz.y);
	let (ix, iz) = UrbanizationExtent::cell_index_containing(position);
	select_kind(UrbanizationExtent::from_cell_index(ix, iz), noise)
}

fn host_at(bounds: Aabb3d) -> MobPlantHost {
	MobPlantHost {
		xz: Vec2::new((bounds.min.x + bounds.max.x) * 0.5, (bounds.min.z + bounds.max.z) * 0.5),
		arrival_radius: urban_leaf_arrival_radius(bounds),
	}
}

impl<U: 'static> ForestSelection for Chico<U> {
	type Read = Res<'static, ForestIndex>;

	fn pick(read: &SystemParamItem<'_, '_, Self::Read>) -> (NoiseParams, Option<LayeringKind>) {
		(read.noise, read.layering)
	}

	fn layers_at(noise: NoiseParams, layering: Option<LayeringKind>, xz: Vec2) -> u8 {
		chico_layers_at(noise, layering, xz)
	}
}

impl<T> PlantHosts for Urbanization<Richmond<T>>
where
	T: RichmondGround,
	Urbanization<Richmond<T>>: TerrainModel<Read = UrbanRead<'static, 'static, Richmond<T>>>,
{
	fn plant_hosts(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<MobPlantHost> {
		let urban: &RichmondRead<'_> = &read.urban;
		let mut hosts = Vec::new();
		for leaf in urban.urbanization.filled_leaves_overlapping(region) {
			hosts.push(host_at(leaf.bounds));
		}
		for cell in urban.developments.filled_cells_overlapping(region) {
			hosts.push(host_at(cell.cell));
		}
		hosts
	}
}

impl<T> UrbanSelection for Urbanization<Richmond<T>>
where
	T: RichmondGround,
	Urbanization<Richmond<T>>: TerrainModel<Read = UrbanRead<'static, 'static, Richmond<T>>>,
{
	fn selection(
		read: &SystemParamItem<'_, '_, Self::Read>,
	) -> (NoiseParams, Option<UrbanizationKind>) {
		let urban: &RichmondRead<'_> = &read.urban;
		(urban.urbanization.noise, urban.urbanization.kind)
	}

	fn kind_at(noise: NoiseParams, pinned: Option<UrbanizationKind>, xz: Vec2) -> UrbanizationKind {
		richmond_kind_at(noise, pinned, xz)
	}
}

impl<T> DiscoverablePlaces for Urbanization<Richmond<T>>
where
	T: RichmondGround,
{
	type Places = (
		Query<'static, 'static, (&'static UrbanSetting, &'static GlobalTransform)>,
		Query<'static, 'static, (&'static DiscoverablePlace, &'static GlobalTransform)>,
	);

	fn places(read: &SystemParamItem<'_, '_, Self::Places>) -> Vec<MobPlantHost> {
		let (settings, places) = read;
		let mut hosts = Vec::new();
		for (setting, transform) in settings.iter() {
			hosts.push(MobPlantHost {
				xz: transform.translation().xz(),
				arrival_radius: setting.arrival_radius,
			});
		}
		for (place, transform) in places.iter() {
			hosts.push(MobPlantHost {
				xz: transform.translation().xz(),
				arrival_radius: place.arrival_radius,
			});
		}
		hosts
	}
}

impl<T> SelectUrbanization for Urbanization<Richmond<T>>
where
	T: RichmondGround,
{
	type Select = ResMut<'static, UrbanizationIndex>;

	fn ensure_selected(select: &mut SystemParamItem<'_, '_, Self::Select>, region: Aabb3d) {
		let noise = select.noise;
		for extent in UrbanizationExtent::cells_overlapping(region) {
			select.ensure_selected(extent, noise);
		}
	}
}
