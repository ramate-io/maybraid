//! [`grove_buckets!`] — declare RFC-shaped grove cell enums from bucket arms.

/// Expand a half-open scalar range literal into [`procedural_common::UnitRange`].
#[macro_export]
macro_rules! unit_range {
	($lo:literal .. $hi:literal) => {
		$crate::unit_range!(@new $lo, $hi)
	};
	(@new $lo:expr, $hi:expr) => {
		procedural_common::UnitRange::new($lo, $hi)
	};
}

/// Expand one authored palette slot (`start..end` token pair).
#[macro_export]
macro_rules! palette_slot {
	($start:ident .. $end:ident) => {
		$crate::grove::PaletteSlot {
			start: $crate::grove::PaletteColor(stringify!($start)),
			end: $crate::grove::PaletteColor(stringify!($end)),
		}
	};
}

/// Expand one authored bucket item payload.
#[macro_export]
macro_rules! grove_bucket_item {
	(
		$item_ty:ident {
			height: $h_lo:literal .. $h_hi:literal,
			width: $w_lo:literal .. $w_hi:literal,
			blade_count: $bc_lo:literal ..= $bc_hi:literal,
			braid_twist: $bt_lo:literal .. $bt_hi:literal,
		}
	) => {
		$item_ty {
			height: $crate::unit_range!($h_lo .. $h_hi),
			width: $crate::unit_range!($w_lo .. $w_hi),
			blade_count: $bc_lo..=$bc_hi,
			braid_twist: $crate::unit_range!($bt_lo .. $bt_hi),
		}
	};
	(
		$item_ty:ident {
			height: $h_lo:literal .. $h_hi:literal,
			width: $w_lo:literal .. $w_hi:literal,
		}
	) => {
		$item_ty {
			height: $crate::unit_range!($h_lo .. $h_hi),
			width: $crate::unit_range!($w_lo .. $w_hi),
		}
	};
	(
		$item_ty:ident {
			height: $h_lo:literal .. $h_hi:literal,
			frond_count: $fc_lo:literal ..= $fc_hi:literal,
			frond_length: $fl_lo:literal .. $fl_hi:literal,
			crown_spread: $cs_lo:literal .. $cs_hi:literal,
		}
	) => {
		$item_ty {
			height: $crate::unit_range!($h_lo .. $h_hi),
			frond_count: $fc_lo..=$fc_hi,
			frond_length: $crate::unit_range!($fl_lo .. $fl_hi),
			crown_spread: $crate::unit_range!($cs_lo .. $cs_hi),
		}
	};
}

/// Declare a grove cell enum with [`Bucket`](crate::grove::Bucket) variants and
/// [`GroveDistribution`](crate::grove::GroveDistribution) builder.
///
/// The `@none` arm must be first; remaining arms are placed variants with `palette_mix` and `item`.
#[macro_export]
macro_rules! grove_buckets {
	(
		$(#[$enum_meta:meta])*
		$vis:vis enum $Enum:ident {
			@none $none_variant:ident {
				weight: $none_weight:literal,
				placement_constraints: $none_constraints:expr,
			},
			$(
				$Variant:ident {
					weight: $weight:literal,
					placement_constraints: $constraints:expr,
					palette_mix: [
						$( [$palette_start:ident .. $palette_end:ident] ),* $(,)?
					],
					item: $item_ty:ident {
						$($item_fields:tt)*
					},
				},
			)*
		}
	) => {
		$(#[$enum_meta])*
		$vis enum $Enum {
			$none_variant($crate::grove::Bucket<()>),
			$(
				$Variant($crate::grove::Bucket<$item_ty>),
			)*
		}

		impl $Enum {
			/// Ordered [`GroveDistribution`] matching macro declaration order.
			pub fn grove_distribution() -> $crate::grove::GroveDistribution<Self> {
				use $crate::grove::{GroveBucket, GroveDistribution};
				let mut dist = GroveDistribution::new();
				dist.push(GroveBucket {
					weight: $none_weight,
					constraints: $none_constraints,
					item: None,
				});
				$(
					dist.push(GroveBucket {
						weight: $weight,
						constraints: $constraints,
						item: Some($Enum::$Variant($crate::grove::Bucket {
							weight: $weight,
							placement_constraints: $constraints,
							palette_mix: $crate::grove::PaletteMix::from_slots(vec![
								$( $crate::palette_slot!($palette_start .. $palette_end) ),*
							]),
							item: $crate::grove_bucket_item!($item_ty {
								$($item_fields)*
							}),
						})),
					});
				)*
				dist
			}
		}
	};
}
