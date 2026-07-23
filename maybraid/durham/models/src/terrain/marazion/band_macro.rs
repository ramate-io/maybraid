//! Dual-band Marazion LOD stack macro (low-pass = small, high-pass = large).
//!
//! Authoring knobs (`cell_size`, `likelihood`, …) live on the call site — same
//! pattern as [`crate::terrain::jersey::family_macro::define_jersey_family`].

/// Defines `PrePocketLayout` → `PrePocketCell` → `PocketCell` → `MarazionLakeCell` for one band.
macro_rules! define_marazion_band {
	(
		layout: $Layout:ident,
		bootstrap_layout: $Bootstrap:ident / $bootstrap_fn:ident,
		pre_cell: $PreCell:ident,
		pocket: $Pocket:ident,
		lake: $LakeCell:ident,
		pre_ids: $pre_ids:ident,
		pocket_ids: $pocket_ids:ident,
		lake_ids: $lake_ids:ident,
		band_field: $band_field:ident,
		band_pass: $band_pass:ident,
		family_salt: $family_salt:expr,
		cell_size: ($cell_min:expr, $cell_max:expr),
		pre_pocket_pitch: $pre_pitch:expr,
		pocket_pitches: $pocket_pitches:expr,
		origin_offset: ($ox:expr, $oz:expr),
		likelihood: $likelihood:expr,
		spatial_correlation: $spatial_correlation:expr,
	) => {
		/// Pre-pocket controller grid for this Marazion band.
		#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq)]
		pub struct $Layout {
			pub grid: $crate::terrain::jersey::shared::OffsetControllerGrid,
		}

		impl Default for $Layout {
			fn default() -> Self {
				Self {
					grid: $crate::terrain::jersey::shared::OffsetControllerGrid::new(
						$pre_pitch,
						bevy::math::Vec2::new($ox, $oz),
					),
				}
			}
		}

		impl $Layout {
			/// Guillotine leaf size lower bound / `min_span` (world units).
			pub const CELL_SIZE_MIN: f32 = $cell_min;
			/// Preferred max leaf / pocket side (world units).
			pub const CELL_SIZE_MAX: f32 = $cell_max;
			/// Pre-pocket controller pitch (world units).
			pub const PRE_POCKET_PITCH: f32 = $pre_pitch;
			/// Discrete pocket pitches (each must divide [`Self::PRE_POCKET_PITCH`]).
			pub const POCKET_PITCHES: [f32; 4] = $pocket_pitches;
			/// World origin offset for this band's controller grid.
			pub const ORIGIN_OFFSET: (f32, f32) = ($ox, $oz);
			/// Default leaf acceptance rate (`0.0..=1.0`).
			pub const LIKELIHOOD: f32 = $likelihood;
			/// Occupancy spatial correlation length (world units).
			pub const SPATIAL_CORRELATION: f32 = $spatial_correlation;
			/// Occupancy / cut salt for this band.
			pub const FAMILY_SALT: u32 = $family_salt;
		}

		pub fn $bootstrap_fn(
			configs: &$crate::terrain::marazion::config::MarazionWatershedConfigs,
		) -> $Layout {
			let band = &configs.$band_field;
			$Layout {
				grid: $crate::terrain::jersey::shared::OffsetControllerGrid::new(
					band.pre_pocket.pitch.max(1.0),
					band.pre_pocket.origin,
				),
			}
		}

		pub trait $Bootstrap {
			fn $bootstrap_fn(&self) -> $Layout;
		}

		impl<S> lod::gen::GenerationScheme<S> for $Layout
		where
			S: $Bootstrap,
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
					spatial_index.$bootstrap_fn(),
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

		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $PreCell {
			pub cell: bevy::math::bounding::Aabb3d,
			pub pre: marazion_watersheds::PrePocket,
		}

		pub fn $pre_ids<S>(
			spatial_index: &mut S,
			region: bevy::math::bounding::Aabb3d,
		) -> Vec<lod::gen::OriginalId>
		where
			S: lod::gen::GeneratingSpatialIndex<$Layout>,
		{
			use bevy::math::bounding::IntersectsVolume;
			use bevy::prelude::*;
			use lod::gen::{GeneratingSpatialIndex, Id, OriginalId, SpatialIndex};
			let identity = Transform::IDENTITY;
			let lod_ref = lod::lod_ref::LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &identity,
				current_transform: &identity,
				bounds: &region,
			};
			if GeneratingSpatialIndex::<$Layout>::get_or_generate(
				spatial_index,
				Id::Universal,
				&lod_ref,
			)
			.is_none()
			{
				return Vec::new();
			}
			let Some(layout) = <S as SpatialIndex<$Layout>>::get(spatial_index, Id::Universal)
			else {
				return Vec::new();
			};
			let grid = layout.grid.clone();
			let grid_region = grid.region_in_grid_space(region);
			$crate::terrain::cell::cell_coords_for_region(grid_region, grid.cell_size)
				.map(|(ix, iz)| OriginalId(Id::from_cell(grid.cell_bounds(ix, iz))))
				.filter(|OriginalId(id)| {
					id.origin_cell_bounds().is_some_and(|b| region.intersects(&b))
				})
				.collect()
		}

		impl<S> lod::gen::GenerationScheme<S> for $PreCell
		where
			S: lod::gen::GeneratingSpatialIndex<
					$crate::terrain::marazion::config::MarazionWatershedConfigs,
				> + lod::gen::GeneratingSpatialIndex<$Layout>,
		{
			fn original_ids_for(
				spatial_index: &mut S,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				$pre_ids(spatial_index, region)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: lod::gen::Id,
				lod_ref: &lod::lod_ref::LodRef,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				use lod::gen::{GeneratingSpatialIndex, Id};
				let cell = id.origin_cell_bounds()?;
				let configs = GeneratingSpatialIndex::<
					$crate::terrain::marazion::config::MarazionWatershedConfigs,
				>::get_one_or_generate(spatial_index, Id::Universal, lod_ref)?;
				let cx = (cell.min.x + cell.max.x) * 0.5;
				let cz = (cell.min.z + cell.max.z) * 0.5;
				let mut params = configs.$band_field.pre_pocket;
				params.seed = configs.seed.wrapping_add(configs.$band_field.family_salt);
				let pre = marazion_watersheds::PrePocket::containing(cx, cz, &params);
				Some((Self { cell, pre }, cell))
			}

			fn descendants_with_lod(
				_id: lod::gen::Id,
				_spatial_index: &mut S,
				_lod_ref: &lod::lod_ref::LodRef,
			) {
			}
		}

		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $Pocket {
			pub cell: bevy::math::bounding::Aabb3d,
			pub leaves: Vec<bevy::math::bounding::Aabb3d>,
		}

		impl $crate::terrain::jersey::shared::LeafAabbs for $Pocket {
			fn leaf_aabbs(&self) -> Vec<bevy::math::bounding::Aabb3d> {
				self.leaves.clone()
			}
		}

		pub fn $pocket_ids<S>(
			spatial_index: &mut S,
			region: bevy::math::bounding::Aabb3d,
		) -> Vec<lod::gen::OriginalId>
		where
			S: lod::gen::GeneratingSpatialIndex<$PreCell>,
		{
			use bevy::math::bounding::IntersectsVolume;
			use bevy::prelude::*;
			use lod::gen::{GeneratingSpatialIndex, Id, OriginalId, SpatialIndex};
			let identity = Transform::IDENTITY;
			let lod_ref = lod::lod_ref::LodRef {
				entity: Entity::PLACEHOLDER,
				previous_transform: &identity,
				current_transform: &identity,
				bounds: &region,
			};
			let pre_cells = GeneratingSpatialIndex::<$PreCell>::get_or_generate_region(
				spatial_index,
				region,
				&lod_ref,
			);
			let mut out = Vec::new();
			for (pre_id, _) in pre_cells {
				let Some(pre_cell) = <S as SpatialIndex<$PreCell>>::get(spatial_index, pre_id)
				else {
					continue;
				};
				let vy_min = pre_cell.cell.min.y;
				let vy_max = pre_cell.cell.max.y;
				for px in 0..pre_cell.pre.nx {
					for pz in 0..pre_cell.pre.nz {
						let aabb = $crate::terrain::marazion::pre_pocket::pocket_aabb(
							&pre_cell.pre,
							px,
							pz,
							vy_min,
							vy_max,
						);
						if region.intersects(&aabb) {
							out.push(OriginalId(Id::from_cell(aabb)));
						}
					}
				}
			}
			out.sort_by(|a, b| a.0.cmp(&b.0));
			out.dedup();
			out
		}

		impl<S> lod::gen::GenerationScheme<S> for $Pocket
		where
			S: lod::gen::GeneratingSpatialIndex<
					$crate::terrain::marazion::config::MarazionWatershedConfigs,
				> + lod::gen::GeneratingSpatialIndex<$PreCell>,
		{
			fn original_ids_for(
				spatial_index: &mut S,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				$pocket_ids(spatial_index, region)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: lod::gen::Id,
				lod_ref: &lod::lod_ref::LodRef,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				use lod::gen::{GeneratingSpatialIndex, Id};
				use procedural_common::Bounds2;
				let cell = id.origin_cell_bounds()?;
				let configs = GeneratingSpatialIndex::<
					$crate::terrain::marazion::config::MarazionWatershedConfigs,
				>::get_one_or_generate(spatial_index, Id::Universal, lod_ref)?;
				let band = &configs.$band_field;
				let mut gparams = band.guillotine;
				gparams.seed = configs.seed.wrapping_add(band.family_salt).wrapping_add(0x6011);
				let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
				let leaves: Vec<_> = marazion_watersheds::guillotine_partition(bounds, &gparams)
					.into_iter()
					.map(|b| {
						$crate::terrain::marazion::pre_pocket::aabb_from_bounds2(
							b,
							cell.min.y,
							cell.max.y,
						)
					})
					.collect();
				Some((Self { cell, leaves }, cell))
			}

			fn descendants_with_lod(
				_id: lod::gen::Id,
				_spatial_index: &mut S,
				_lod_ref: &lod::lod_ref::LodRef,
			) {
			}
		}

		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $LakeCell {
			pub cell: bevy::math::bounding::Aabb3d,
			pub kind: $crate::terrain::marazion::leaf_kind::MarazionLeafKind,
			pub band: $crate::terrain::marazion::leaf_kind::MarazionBandPass,
			/// Authored watershed graph; compiled when pulled into [`crate::terrain::Terrain`].
			pub complex: marazion_watersheds::WatershedDepressionComplex,
		}

		pub fn $lake_ids<S>(
			spatial_index: &mut S,
			region: bevy::math::bounding::Aabb3d,
		) -> Vec<lod::gen::OriginalId>
		where
			S: lod::gen::GeneratingSpatialIndex<$Pocket>,
		{
			$crate::terrain::jersey::shared::original_ids_for_leaves::<S, $Pocket>(
				spatial_index,
				region,
			)
		}

		impl<S> lod::gen::GenerationScheme<S> for $LakeCell
		where
			S: lod::gen::GeneratingSpatialIndex<
					$crate::terrain::marazion::config::MarazionWatershedConfigs,
				> + lod::gen::GeneratingSpatialIndex<$Pocket>
				+ lod::gen::GeneratingSpatialIndex<$crate::terrain::PreWatershedTerrain>
				+ lod::gen::GeneratingSpatialIndex<$crate::terrain::cell::TerrainCellLayout>,
		{
			fn original_ids_for(
				spatial_index: &mut S,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				$lake_ids(spatial_index, region)
			}

			fn build_with_id(
				spatial_index: &mut S,
				id: lod::gen::Id,
				lod_ref: &lod::lod_ref::LodRef,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				use lod::gen::{GeneratingSpatialIndex, Id};
				use procedural_common::Bounds2;
				let cell = id.origin_cell_bounds()?;
				let configs = GeneratingSpatialIndex::<
					$crate::terrain::marazion::config::MarazionWatershedConfigs,
				>::get_one_or_generate(spatial_index, Id::Universal, lod_ref)?
				.clone();
				let band = configs.$band_field.clone();
				let occ_seed = $crate::terrain::jersey::shared::occupancy_seed(
					configs.seed,
					0,
					band.family_salt,
				);
				let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
				let seed = configs.seed.wrapping_add(band.family_salt).wrapping_add(
					cell.min.x.to_bits().wrapping_mul(73856093)
						^ cell.min.z.to_bits().wrapping_mul(19349663),
				);
				let empty_cell = || Self {
					cell,
					kind: $crate::terrain::marazion::leaf_kind::MarazionLeafKind::Empty,
					band: $crate::terrain::marazion::leaf_kind::MarazionBandPass::$band_pass,
					complex: marazion_watersheds::WatershedDepressionComplex::new(bounds, seed),
				};
				if !$crate::terrain::jersey::shared::leaf_selected(
					cell,
					occ_seed,
					band.likelihood,
					band.spatial_correlation,
				) {
					return Some((empty_cell(), cell));
				}

				// Live pre-watershed sampler: each shelf / endpoint survey point may
				// land in a different terrain origin cell.
				let spatial_index = std::cell::RefCell::new(spatial_index);
				let height_fn = |x: f32, z: f32| {
					$crate::terrain::marazion::height::pre_watershed_height_at(
						*spatial_index.borrow_mut(),
						x,
						z,
						lod_ref,
					)
					.unwrap_or(0.0)
				};
				let height_at: Option<&dyn Fn(f32, f32) -> f32> = Some(&height_fn);

				// Occupied leaves: stream / streams-graph / bog / lake from a stable unit draw.
				let type_u =
					procedural_common::SeededHash::new(seed.wrapping_add(0x57EA_71FE)).unit(0);
				let stream_cut = band.stream_frac.clamp(0.0, 1.0);
				let graph_cut =
					(stream_cut + band.streams_graph_frac.clamp(0.0, 1.0)).min(1.0);
				let bog_cut = (graph_cut + band.bog_frac.clamp(0.0, 1.0)).min(1.0);
				let prefer = if type_u < stream_cut {
					0u8 // stream
				} else if type_u < graph_cut {
					1u8 // streams graph
				} else if type_u < bog_cut {
					2u8 // bog
				} else {
					3u8 // lake
				};

				type LeafStamp = (
					$crate::terrain::marazion::leaf_kind::MarazionLeafKind,
					marazion_watersheds::WatershedDepressionComplex,
				);
				let try_stream = || -> Option<LeafStamp> {
					marazion_watersheds::Stream::from_bounds(
						bounds,
						seed,
						band.stream,
						height_at,
					)
					.map(|stream| {
						(
							$crate::terrain::marazion::leaf_kind::MarazionLeafKind::Stream,
							stream.into_complex(),
						)
					})
				};
				let try_streams_graph = || -> Option<LeafStamp> {
					marazion_watersheds::StreamsGraph::from_bounds(
						bounds,
						seed,
						band.streams_graph,
						height_at,
					)
					.map(|graph| {
						(
							$crate::terrain::marazion::leaf_kind::MarazionLeafKind::StreamsGraph,
							graph.into_complex(),
						)
					})
				};
				let try_bog = || -> Option<LeafStamp> {
					marazion_watersheds::Bog::from_bounds(
						bounds,
						seed,
						band.bog,
						height_at,
					)
					.map(|bog| {
						(
							$crate::terrain::marazion::leaf_kind::MarazionLeafKind::Bog,
							bog.into_complex(),
						)
					})
				};
				let try_lake = || -> Option<LeafStamp> {
					marazion_watersheds::Lake::from_bounds(
						bounds,
						seed,
						band.lake,
						height_at,
					)
					.map(|lake| {
						(
							$crate::terrain::marazion::leaf_kind::MarazionLeafKind::Lake,
							lake.into_complex(),
						)
					})
				};

				let empty_stamp = || {
					(
						$crate::terrain::marazion::leaf_kind::MarazionLeafKind::Empty,
						marazion_watersheds::WatershedDepressionComplex::new(bounds, seed),
					)
				};
				let (kind, complex) = match prefer {
					0 => try_stream()
						.or_else(try_streams_graph)
						.or_else(try_bog)
						.or_else(try_lake)
						.unwrap_or_else(empty_stamp),
					1 => try_streams_graph()
						.or_else(try_stream)
						.or_else(try_bog)
						.or_else(try_lake)
						.unwrap_or_else(empty_stamp),
					2 => try_bog()
						.or_else(try_stream)
						.or_else(try_streams_graph)
						.or_else(try_lake)
						.unwrap_or_else(empty_stamp),
					_ => try_lake()
						.or_else(try_stream)
						.or_else(try_streams_graph)
						.or_else(try_bog)
						.unwrap_or_else(empty_stamp),
				};

				if complex.is_empty() {
					return Some((empty_cell(), cell));
				}
				Some((
					Self {
						cell,
						kind,
						band: $crate::terrain::marazion::leaf_kind::MarazionBandPass::$band_pass,
						complex,
					},
					cell,
				))
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

pub(crate) use define_marazion_band;
