//! [`bucket_throw!`] — declare a weighted variant enum and its throw distribution.

/// Declare an ordered weighted distribution and wire variant types through [`FromScalarNoise`].
///
/// # Example
///
/// ```ignore
/// bucket_throw! {
///     pub enum Tree {
///         Oak(Oak) => 1.0,
///         Pine(Pine) => 2.0,
///         Birch(Birch) => 3.0,
///     }
/// }
/// ```
///
/// Expands to the enum, a `{Enum}Builder` marker enum, [`BuildWithNoise`] for the builder,
/// and [`crate::distributions::bucket_throw::TypedBucketThrow`] via [`{Enum}::bucket_throw()`].
#[macro_export]
macro_rules! bucket_throw {
	(
		$(#[$enum_meta:meta])*
		$vis:vis enum $enum_name:ident {
			$(
				$(#[$variant_meta:meta])*
				$variant:ident ( $inner:ty ) => $weight:expr
			),* $(,)?
		}
	) => {
		::paste::paste! {
			$(#[$enum_meta])*
			$vis enum $enum_name {
				$(
					$(#[$variant_meta])*
					$variant($inner),
				)*
			}

			#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
			$vis enum [< $enum_name Builder >] {
				$(
					$variant,
				)*
			}

			impl $crate::noise::BuildWithNoise<$enum_name> for [< $enum_name Builder >] {
				fn build_with_noise(&self, noise: $crate::NoiseParams) -> $enum_name {
					match self {
						$(
							Self::$variant => {
								$enum_name::$variant(
									<$inner as $crate::FromScalarNoise>::from_scalar(noise),
								)
							}
						)*
					}
				}
			}

			impl $enum_name {
				$vis fn bucket_throw() -> $crate::distributions::bucket_throw::TypedBucketThrow<
					[< $enum_name Builder >],
				> {
					let mut distribution =
						$crate::distributions::bucket_throw::TypedBucketThrow::new();
					$(
						distribution.add([< $enum_name Builder >]::$variant, $weight);
					)*
					distribution
				}
			}
		}
	};
}
