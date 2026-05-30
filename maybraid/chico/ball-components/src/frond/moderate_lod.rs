//! Moderate-LOD frond mesh (shoot tube + lateral leaflet cards for ~30–50 m viewing).

mod construction;
mod frond_crown;
mod leaflet;

pub use construction::{ModerateLodPalmFrondCluster, ModerateLodPalmFrondElement};
pub use frond_crown::{
	ModerateLodFrondCrown, ModerateLodFrondCrownShape, ModerateLodFrondCrownStd,
	ModerateLodPalmFrond,
};
