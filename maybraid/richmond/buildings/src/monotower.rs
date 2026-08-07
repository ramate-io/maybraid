//! A monotower is a tower of a single type of storey floor plan.
//!
//! Because the mappings of shafts onto the floor plans to the storeys are one-to-one,
//! we can ensure good continuity.
//!
//! The monotower has the responsibility of filling in proper shafts.
//!
//! When monotowers fit and AaBb3d, they choose a floor height from a range
//! and then adjust to have a whole number of storeys.
//!
//! Monotowers typically follow a Parameterized -> FloorPlan -> Full approach as well
//! wherein floor plans include storey floor plans and stairs in the shafts. Then usage
//! areas are filled using the same pattern as a particular full variant per storey.
//!
//! Note that, going forward, we should probably make clearer the *UsagePlan concept in stories as well,
//! breaking out the UsagePlan paint-on logic from the full-storey struct itself in a nice
//! reusable way.
