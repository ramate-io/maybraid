//! Shared Penmarch / Kamakura stick and canopy helpers.
//!
//! Both torches use [`StorybookTreeChain`](chico_sbs_geometry::StorybookTreeChain) with the
//! same selective upper/outer layered-ball foliage policy; this module holds that emission
//! logic once.

mod canopy;
mod stick;

pub(crate) use canopy::{
	foliage_nodes_banded, foliage_nodes_high, LOW_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};
pub(crate) use stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium};
