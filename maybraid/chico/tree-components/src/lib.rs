//! Common (sub)component assemblies for Chico vegetation ([#226](https://github.com/ramate-io/maybraid/issues/226)).
pub mod braid_oak_canopy;
pub mod high_bush_shoots;
pub mod jungle_growth;
pub mod jungle_storybook_canopy;
pub mod skipped_mesh_material;

pub use high_bush_shoots::{
	apply_common_high_bush_preset, should_allocate_foliage, HighBushFoliageStyle, HighBushShoots,
	HighBushShootsShape, COMMON_HIGH_BUSH_RADIAL_STRENGTH, COMMON_HIGH_BUSH_SHOOT_COUNT,
	COMMON_HIGH_BUSH_VERTICAL_BIAS,
};
pub use jungle_growth::{JungleGrowth, JungleGrowthShape};
pub use jungle_storybook_canopy::JungleStorybookCanopyFoliage;

/// Honu Banyan reuses the same three-variant canopy payload as Jungle Storybook ([#250](https://github.com/ramate-io/maybraid/issues/250)).
pub type HonuBanyanCanopyFoliage<InnerM, InnerS, OuterM, OuterS, BodyM, BodyS, FoliageM, FoliageS> =
	JungleStorybookCanopyFoliage<InnerM, InnerS, OuterM, OuterS, BodyM, BodyS, FoliageM, FoliageS>;
pub use braid_oak_canopy::BraidOakCanopyFoliage;
pub use skipped_mesh_material::{SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial};
