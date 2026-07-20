//! Expand one jersey family: controller layout + controller cell + leaf stamp.

/// Defines an independent guillotine stack for one stamp family.
///
/// Each family owns its own controller grid (`cell_size` / `origin_offset`) and
/// cut seed (via [`crate::terrain::jersey::configs::JerseyStampConfigs`]). Leaf
/// identities are not stored: stamp `build_with_id` down-levels `Id` to cell
/// bounds. Discovery walks that family's controllers only.
macro_rules! define_jersey_family {
	(
		layout: $Layout:ident,
		bootstrap_layout: $BootstrapLayout:ident / $bootstrap_layout_fn:ident,
		controller: $Controller:ident,
		stamp: $Stamp:ident,
		leaves_fn: $leaves_fn:ident,
		family_salt: $family_salt:expr,
		cell_size: $cell_size:expr,
		origin_offset: ($ox:expr, $oz:expr),
		config_field: $config_field:ident,
		|$bounds:ident, $seed:ident, $height_at:ident, $params:ident| $build:expr
	) => {
		/// Controller-grid layout for this jersey family.
		#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq)]
		pub struct $Layout {
			pub grid: $crate::terrain::jersey::shared::OffsetControllerGrid,
		}

		impl Default for $Layout {
			fn default() -> Self {
				Self {
					grid: $crate::terrain::jersey::shared::OffsetControllerGrid::new(
						$cell_size,
						bevy::math::Vec2::new($ox, $oz),
					),
				}
			}
		}

		impl $Layout {
			pub fn cell_bounds(&self, ix: i32, iz: i32) -> bevy::math::bounding::Aabb3d {
				self.grid.cell_bounds(ix, iz)
			}

			pub fn region_in_grid_space(
				&self,
				region: bevy::math::bounding::Aabb3d,
			) -> bevy::math::bounding::Aabb3d {
				self.grid.region_in_grid_space(region)
			}
		}

		pub trait $BootstrapLayout {
			fn $bootstrap_layout_fn(&self) -> $Layout;
		}

		impl<S> lod::gen::GenerationScheme<S> for $Layout
		where
			S: $BootstrapLayout,
		{
			fn original_ids_for(
				_spatial_index: &mut S,
				_region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				vec![lod::gen::OriginalId::universal()]
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: lod::gen::Id,
				_lod_ref: &lod::lod_ref::LodRef,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				if id != lod::gen::Id::Universal {
					return None;
				}
				Some((
					spatial_index.$bootstrap_layout_fn(),
					$crate::terrain::cell::universal_bounds(),
				))
			}

			fn descendants_with_lod(
				_id: lod::gen::Id,
				_spatial_index: &mut S,
				_lod_ref: &lod::lod_ref::LodRef,
			) {
			}
		}

		/// Controller cell: owns this family's guillotine cuts.
		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $Controller {
			pub cell: bevy::math::bounding::Aabb3d,
			pub cuts: comproc::guillotine::GuillotineCuts<2>,
		}

		impl $Controller {
			pub fn from_family_config<P>(
				cell: bevy::math::bounding::Aabb3d,
				config: &$crate::terrain::jersey::configs::FamilyGuillotineConfig<P>,
			) -> Self {
				Self {
					cell,
					cuts: $crate::terrain::jersey::shared::guillotine_cuts(cell, config),
				}
			}
		}

		impl $crate::terrain::jersey::shared::LeafAabbs for $Controller {
			fn leaf_aabbs(&self) -> Vec<bevy::math::bounding::Aabb3d> {
				$crate::terrain::jersey::shared::leaf_aabbs(self.cell, &self.cuts)
			}
		}

		impl<S> lod::gen::GenerationScheme<S> for $Controller
		where
			S: lod::gen::GeneratingSpatialIndex<$crate::terrain::jersey::configs::JerseyStampConfigs>
				+ lod::gen::GeneratingSpatialIndex<$Layout>,
		{
			fn original_ids_for(
				spatial_index: &mut S,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				$crate::terrain::jersey::shared::original_ids_for_controller_cells::<S, $Layout>(
					spatial_index,
					region,
					|layout| &layout.grid,
				)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: lod::gen::Id,
				lod_ref: &lod::lod_ref::LodRef,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				let bounds = id.origin_cell_bounds()?;
				let configs = lod::gen::GeneratingSpatialIndex::<
					$crate::terrain::jersey::configs::JerseyStampConfigs,
				>::get_one_or_generate(spatial_index, lod::gen::Id::Universal, lod_ref)?;
				let family = &configs.$config_field;
				Some((Self::from_family_config(bounds, family), bounds))
			}

			fn descendants_with_lod(
				_id: lod::gen::Id,
				_spatial_index: &mut S,
				_lod_ref: &lod::lod_ref::LodRef,
			) {
			}
		}

		/// Discover leaf ids for this family's controller grid.
		pub fn $leaves_fn<S>(
			spatial_index: &mut S,
			region: bevy::math::bounding::Aabb3d,
		) -> Vec<lod::gen::OriginalId>
		where
			S: lod::gen::GeneratingSpatialIndex<$Controller>,
		{
			$crate::terrain::jersey::shared::original_ids_for_leaves::<S, $Controller>(
				spatial_index,
				region,
			)
		}

		/// Stamp output on one leaf of this family's guillotine partition.
		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $Stamp {
			pub cell: bevy::math::bounding::Aabb3d,
			pub modulations: Vec<jersey_terrain_stamps::JerseyModulation>,
		}

		impl<S> lod::gen::GenerationScheme<S> for $Stamp
		where
			S: lod::gen::GeneratingSpatialIndex<$crate::terrain::jersey::configs::JerseyStampConfigs>
				+ lod::gen::GeneratingSpatialIndex<$crate::terrain::base_noise::BaseTerrainNoise>
				+ lod::gen::GeneratingSpatialIndex<$Controller>,
		{
			fn original_ids_for(
				spatial_index: &mut S,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				$leaves_fn(spatial_index, region)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: lod::gen::Id,
				lod_ref: &lod::lod_ref::LodRef,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				let cell = id.origin_cell_bounds()?;
				let configs = lod::gen::GeneratingSpatialIndex::<
					$crate::terrain::jersey::configs::JerseyStampConfigs,
				>::get_one_or_generate(spatial_index, lod::gen::Id::Universal, lod_ref)?
				.clone();
				let base = lod::gen::GeneratingSpatialIndex::<
					$crate::terrain::base_noise::BaseTerrainNoise,
				>::get_one_or_generate(spatial_index, lod::gen::Id::Universal, lod_ref)?;
				let family = &configs.$config_field;
				let $bounds = $crate::terrain::jersey::shared::bounds2(cell);
				let $seed = $crate::terrain::jersey::shared::family_seed(
					base.seed,
					cell,
					$family_salt,
				);
				let $params = family.stamp.clone();
				let height = |x: f32, z: f32| base.height_at(x, z);
				let $height_at: Option<&dyn Fn(f32, f32) -> f32> = Some(&height);
				let modulations = $build;
				Some((Self { cell, modulations }, cell))
			}

			fn descendants_with_lod(
				_id: lod::gen::Id,
				_spatial_index: &mut S,
				_lod_ref: &lod::lod_ref::LodRef,
			) {
			}
		}
	};
}

pub(crate) use define_jersey_family;
