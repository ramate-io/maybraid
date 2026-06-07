//! [`CellGrove`] — authored grove identity ([RFC-183 3.4]).

use super::{distribution::GroveDistribution, params::GroveParamRanges};

/// Authored grove identity: parameter ranges plus ordered variant distribution.
pub trait CellGrove {
	type Variant: Clone;

	fn param_ranges(&self) -> GroveParamRanges;

	fn distribution(&self) -> &GroveDistribution<Self::Variant>;
}
