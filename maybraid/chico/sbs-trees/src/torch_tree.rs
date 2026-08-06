//! Shared stick / foliage emission for Penmarch and Kamakura torch trees
//! (same [`StorybookTreeChain`] topology; selective upper/outer canopy).

mod canopy;
mod stick;

pub(crate) use canopy::{
	foliage_nodes_banded, foliage_nodes_high, LOW_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};
pub(crate) use stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium_banded};
